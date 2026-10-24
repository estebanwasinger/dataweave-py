use super::super::extended::{Entry, Expression, Key, MatchCase, Part, Pattern};
use super::*;

impl Parser<'_> {
    pub(super) fn using(&self, s: &str, depth: usize) -> Result<Kind, DwError> {
        let rest = s.strip_prefix("using").unwrap().trim_start();
        if !rest.starts_with('(') {
            return Err(self.error(rest, "expected using bindings"));
        }
        let close = find_matching_delimiter(rest, 0, '(', ')')
            .ok_or_else(|| self.error(rest, "unterminated using bindings"))?;
        let mut declarations = vec![];
        for binding in split_top_level(&rest[1..close], ',')
            .into_iter()
            .filter(|b| !b.trim().is_empty())
        {
            let (name, value) = split_top_level_char(binding, '=')
                .ok_or_else(|| self.error(binding, "using binding requires ="))?;
            let name = name.split(':').next().unwrap().trim();
            if !is_identifier(name) {
                return Err(self.error(binding, "invalid using binding"));
            }
            declarations.push(Declaration::Var(
                name.into(),
                self.expr(value, true, depth + 1)?,
            ));
        }
        let scoped = self.scoped(declarations.iter().filter_map(|d| {
            if let Declaration::Var(n, _) = d {
                Some(n.clone())
            } else {
                None
            }
        }));
        Ok(Kind::Block(
            declarations,
            scoped.expr(&rest[close + 1..], true, depth + 1)?,
        ))
    }
    pub(super) fn match_expression(
        &self,
        value: &str,
        cases: &str,
        depth: usize,
    ) -> Result<Kind, DwError> {
        let mut out = vec![];
        let sources = crate::matches::split_match_cases(cases);
        if !cases.trim().is_empty()
            && sources.first().map(|s| s.as_ptr()) != Some(cases.trim().as_ptr())
        {
            return Err(self.error(cases, "expected match case"));
        }
        for source in sources {
            let (pattern, body) = split_top_level_arrow(source)
                .ok_or_else(|| self.error(source, "match case requires ->"))?;
            let (pattern, guard) = if pattern.trim() == "else" {
                (Pattern::Any(None), None)
            } else {
                let pattern = pattern
                    .trim()
                    .strip_prefix("case ")
                    .ok_or_else(|| self.error(pattern, "expected case or else"))?
                    .trim();
                let (pattern, guard) = split_top_level_keyword(pattern, "when")
                    .or_else(|| {
                        split_top_level_keyword_or_call_operator(pattern, &["if"])
                            .map(|(a, _, b)| (a, b))
                    })
                    .map_or((pattern, None), |(a, b)| (a, Some(b)));
                let pattern = if let Some(binding) = pattern.strip_prefix("var ") {
                    Pattern::Any(Some(binding.trim().into()))
                } else if let Some(t) = pattern.strip_prefix("is ") {
                    Pattern::Type(None, t.into())
                } else if let Some((binding, regex)) = split_top_level_keyword(pattern, "matches") {
                    Pattern::Regex(binding.into(), self.expr(regex, false, depth + 1)?)
                } else if let Some((binding, t)) = split_top_level_keyword(pattern, "is") {
                    Pattern::Type(Some(binding.into()), t.into())
                } else if let Some((binding, expected)) = split_top_level_char(pattern, ':') {
                    Pattern::Equals(Some(binding.into()), self.expr(expected, false, depth + 1)?)
                } else if is_identifier(pattern) && !matches!(pattern, "true" | "false" | "null") {
                    Pattern::Any(Some(pattern.into()))
                } else {
                    Pattern::Equals(None, self.expr(pattern, false, depth + 1)?)
                };
                if let Some(binding) = pattern.binding() {
                    if !is_identifier(binding) {
                        return Err(self.error(source, "invalid match binding"));
                    }
                }
                (pattern, guard)
            };
            out.push(MatchCase {
                pattern,
                guard: guard.map(|g| self.expr(g, true, depth + 1)).transpose()?,
                body: self.expr(body, true, depth + 1)?,
            });
        }
        Ok(Kind::Extra(Expression::Match(
            self.expr(value, false, depth + 1)?,
            Rc::new(out),
        )))
    }
    pub(super) fn object(&self, s: &str, depth: usize) -> Result<Kind, DwError> {
        let mut out = vec![];
        let entries = crate::evaluator::split_object_entries(&s[1..s.len() - 1]);
        for (i, source) in entries.iter().enumerate() {
            if source.trim().is_empty() {
                if i + 1 == entries.len() {
                    continue;
                }
                return Err(self.error(source, "empty object entry"));
            }
            let (entry, guard) = crate::evaluator::parse_conditional_literal_element(source)
                .map_or((source.trim(), None), |(a, b)| {
                    (strip_wrapping_parens(a), Some(b))
                });
            let (key, value) = if let Some((key, value)) = split_top_level_char(entry, ':') {
                let key = key.trim();
                let key = if is_identifier(key) {
                    Key::Static(key.into())
                } else if self.is_whole_string(key) && !key.contains('$') {
                    Key::Static(crate::literals::parse_string_literal(key)?.unwrap())
                } else if key.contains("@(") {
                    Key::Static(crate::literals::parse_object_key(key)?)
                } else {
                    Key::Computed(self.expr(strip_wrapping_parens(key), true, depth + 1)?)
                };
                (Some(key), value)
            } else {
                (None, entry)
            };
            out.push(Entry {
                key,
                value: self.expr(value, true, depth + 1)?,
                guard: guard.map(|g| self.expr(g, true, depth + 1)).transpose()?,
                source: entry.into(),
            });
        }
        if out
            .iter()
            .all(|e| e.guard.is_none() && matches!(e.key, Some(Key::Static(_))))
        {
            return Ok(Kind::Object(
                out.into_iter()
                    .map(|e| {
                        let Some(Key::Static(key)) = e.key else {
                            unreachable!()
                        };
                        (key, e.value)
                    })
                    .collect(),
            ));
        }
        Ok(Kind::Extra(Expression::Object(Rc::new(out))))
    }
    pub(super) fn is_whole_string(&self, s: &str) -> bool {
        let Some(quote) = s
            .as_bytes()
            .first()
            .copied()
            .filter(|q| matches!(q, b'"' | b'\'' | b'`'))
        else {
            return false;
        };
        let mut i = 1;
        while i < s.len() {
            let c = s.as_bytes()[i];
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'$' && s.as_bytes().get(i + 1) == Some(&b'(') {
                let Some(close) = find_matching_delimiter(s, i + 1, '(', ')') else {
                    return false;
                };
                i = close + 1;
                continue;
            }
            if c == quote {
                return i == s.len() - 1;
            }
            i += 1;
        }
        false
    }
    pub(super) fn interpolation(
        &self,
        s: &str,
        depth: usize,
    ) -> Result<Option<Vec<Part>>, DwError> {
        if !self.is_whole_string(s) {
            return Ok(None);
        }
        let quote = s.as_bytes()[0] as char;
        let mut parts = vec![];
        let mut start = 1;
        let mut i = 1;
        while i < s.len() - 1 {
            if s.as_bytes()[i] == b'\\' {
                i += 2;
                continue;
            }
            if s.as_bytes()[i] == b'$' && s.as_bytes().get(i + 1) == Some(&b'(') {
                if start < i {
                    parts.push(Part::Literal(format!("{quote}{}{quote}", &s[start..i])));
                }
                let end = find_matching_delimiter(s, i + 1, '(', ')')
                    .ok_or_else(|| self.error(&s[i..], "unterminated interpolation"))?;
                parts.push(Part::Value(self.expr(&s[i + 2..end], true, depth + 1)?));
                i = end + 1;
                start = i;
            } else {
                i += 1;
            }
        }
        if parts.is_empty() {
            return Ok(None);
        }
        if start < s.len() - 1 {
            parts.push(Part::Literal(format!(
                "{quote}{}{quote}",
                &s[start..s.len() - 1]
            )));
        }
        Ok(Some(parts))
    }
    pub(super) fn replace_parts<'a>(&self, s: &'a str) -> Option<(&'a str, &'a str, &'a str)> {
        if let Some((input, right)) = split_top_level_keyword(s, "replace") {
            let (target, _, replacement) =
                split_top_level_keyword_or_call_operator(right, &["with"])?;
            return Some((input, target, replacement));
        }
        let (left, _, replacement) = split_top_level_keyword_or_call_operator(s, &["with"])?;
        let (name, args) = parse_call_args(left)?;
        (name == "replace" && args.len() == 2).then(|| (args[0], args[1], replacement))
    }
    pub(super) fn selector(
        &self,
        base: &str,
        index: &str,
        depth: usize,
    ) -> Result<Option<Kind>, DwError> {
        let index = index.trim();
        let Some(prefix) = index
            .as_bytes()
            .first()
            .copied()
            .filter(|c| matches!(c, b'?' | b'*' | b'&'))
        else {
            return Ok(None);
        };
        if !index[1..].starts_with('(')
            || find_matching_delimiter(index, 1, '(', ')') != Some(index.len() - 1)
        {
            return Ok(None);
        }
        let mut base = self.expr(base, false, depth + 1)?;
        if let Kind::Select(input, path) = &base.kind {
            base = Rc::new(Node {
                range: base.range,
                stop: base.stop,
                kind: Kind::Extra(Expression::RawSelect(input.clone(), path.clone())),
            });
        }
        Ok(Some(Kind::Extra(Expression::Selector(
            base,
            prefix,
            self.expr(&index[2..index.len() - 1], true, depth + 1)?,
        ))))
    }
}
