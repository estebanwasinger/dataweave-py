//! Source-preserving debug grammar. All child spans come from borrowed slices
//! of one comment-masked source buffer, never from searching expression text.
use super::SourceRange;
use crate::{
    literals::{parse_call_args, parse_literal},
    syntax::*,
    DwError,
};
use std::rc::Rc;

pub(super) type Expr = Rc<Node>;
#[derive(Clone)]
pub(super) struct Node {
    pub range: SourceRange,
    pub stop: bool,
    pub kind: Kind,
}
#[derive(Clone)]
pub(super) enum Kind {
    Extra(super::extended::Expression),
    Atom(String),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Binary(String, Expr, Expr),
    Unary(String, Expr),
    Type(String, Expr, String),
    If(Expr, Expr, Expr),
    Call(String, Vec<Expr>),
    Select(Expr, String),
    Index(Expr, Expr),
    Slice(Expr, Expr, Expr),
    Lambda(Vec<Param>, Expr),
    Collection(String, Expr, Expr),
    Block(Vec<Declaration>, Expr),
    Update(Expr, Rc<Vec<UpdateCase>>),
    UpdateWith(Expr, Expr, Expr),
}
#[derive(Clone)]
pub(super) struct UpdateCase {
    pub binding: String,
    pub path: String,
    pub guard: Option<Expr>,
    pub body: Expr,
}
#[derive(Clone)]
pub(super) struct Param {
    pub name: String,
    pub type_source: Option<String>,
    pub default: Option<Expr>,
}
#[derive(Clone)]
pub(super) enum Declaration {
    Var(String, Expr),
    Fun(String, Vec<Param>, Expr),
    Directive(String),
}
pub(super) struct Program {
    pub root: Expr,
    pub masked: String,
    pub output: Option<String>,
    pub lazy: bool,
}
mod extended;
struct Parser<'a> {
    source: &'a str,
    aliases: crate::DwMap<String, crate::DwValue>,
    shadowed: std::collections::BTreeSet<String>,
}

pub(super) fn parse(source: &str) -> Result<Program, DwError> {
    let masked = mask_comments(source)?;
    let (header, body) = if let Some((a, b)) = crate::parse_script_boundary_span(&masked) {
        (&masked[..a], &masked[b..])
    } else {
        (&masked[..0], masked.as_str())
    };
    let mut aliases = crate::DwMap::new();
    for declaration in crate::functions::collect_header_declarations(header) {
        if declaration.trim().starts_with("import ") {
            crate::functions::register_import_aliases(declaration.trim(), &mut aliases)?;
        }
    }
    let mut p = Parser {
        source: &masked,
        aliases,
        shadowed: Default::default(),
    };
    p.header_bindings(header);
    let declarations = p.header(header)?;
    let output = header
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("output ").map(str::to_owned));
    let lazy = crate::compiled::compile_body(body.trim()).is_some();
    let body = p.expr(body, true, 0)?;
    let root = Rc::new(Node {
        range: body.range,
        stop: false,
        kind: Kind::Block(declarations, body),
    });
    Ok(Program {
        root,
        masked,
        output,
        lazy,
    })
}
pub(super) fn parse_watch(source: &str) -> Result<Expr, DwError> {
    Ok(parse(source)?.root)
}
impl Parser<'_> {
    fn scoped(&self, names: impl IntoIterator<Item = String>) -> Self {
        let mut shadowed = self.shadowed.clone();
        shadowed.extend(names);
        Self {
            source: self.source,
            aliases: self.aliases.clone(),
            shadowed,
        }
    }
    fn header_bindings(&mut self, source: &str) {
        for declaration in crate::functions::collect_header_declarations(source) {
            let s = declaration.trim();
            if let Some(rest) = s.strip_prefix("fun ") {
                if let Some(open) = rest.find('(') {
                    self.shadowed.insert(
                        crate::functions::strip_generic_function_type_parameters(
                            rest[..open].trim(),
                        )
                        .into(),
                    );
                }
            } else if let Some(rest) = s.strip_prefix("var ") {
                if let Some((name, _)) = split_top_level_char(rest, '=') {
                    self.shadowed
                        .insert(name.split(':').next().unwrap().trim().into());
                }
            }
        }
    }
    fn range(&self, s: &str) -> SourceRange {
        SourceRange {
            start: s.as_ptr() as usize - self.source.as_ptr() as usize,
            end: s.as_ptr() as usize - self.source.as_ptr() as usize + s.len(),
        }
    }
    fn error(&self, s: &str, message: &str) -> DwError {
        let r = self.range(s);
        let (line, column) = super::position(self.source, r.start);
        DwError::Parse(format!("Debug: {message} at {line}:{column}"))
    }
    fn params(&self, source: &str, depth: usize) -> Result<Vec<Param>, DwError> {
        let source = strip_wrapping_parens(source.trim());
        if source.is_empty() {
            return Ok(vec![]);
        }
        split_top_level(source, ',')
            .into_iter()
            .map(|s| {
                let (name, default) =
                    split_top_level_char(s, '=').map_or((s, None), |(a, b)| (a, Some(b)));
                let (name, type_source) = name
                    .split_once(':')
                    .map_or((name, None), |(name, t)| (name, Some(t.trim().to_owned())));
                let name = name.trim();
                if !is_identifier(name) {
                    return Err(self.error(s, "invalid parameter"));
                }
                Ok(Param {
                    name: name.into(),
                    type_source,
                    default: default
                        .map(|s| self.expr(s, false, depth + 1))
                        .transpose()?,
                })
            })
            .collect()
    }
    fn header(&self, source: &str) -> Result<Vec<Declaration>, DwError> {
        let mut starts = vec![];
        let mut offset = 0;
        for line in source.split_inclusive('\n') {
            let s = line.trim_start();
            let i = offset + line.len() - s.len();
            if !s.trim().is_empty()
                && is_top_level_index(source, i)
                && [
                    "%dw", "output ", "var ", "fun ", "import ", "input ", "type ", "ns ",
                ]
                .iter()
                .any(|prefix| s.starts_with(prefix))
            {
                starts.push(i);
            }
            offset += line.len();
        }
        if starts.is_empty() && !source.trim().is_empty() {
            return Err(self.error(source, "unsupported header"));
        }
        if starts
            .first()
            .is_some_and(|first| !source[..*first].trim().is_empty())
        {
            return Err(self.error(source, "unsupported header prefix"));
        }
        starts.push(source.len());
        let mut result = vec![];
        for pair in starts.windows(2) {
            let s = source[pair[0]..pair[1]].trim();
            if let Some(rest) = s.strip_prefix("var ") {
                let (name, value) = split_top_level_char(rest, '=')
                    .ok_or_else(|| self.error(s, "expected var name = expression"))?;
                let name = name.split(':').next().unwrap_or("").trim();
                if !is_identifier(name) {
                    return Err(self.error(s, "invalid variable"));
                }
                result.push(Declaration::Var(name.into(), self.expr(value, true, 0)?));
            } else if let Some(rest) = s.strip_prefix("fun ") {
                let open = rest
                    .find('(')
                    .ok_or_else(|| self.error(s, "invalid function"))?;
                let close = find_matching_delimiter(rest, open, '(', ')')
                    .ok_or_else(|| self.error(s, "invalid function parameters"))?;
                let name =
                    crate::functions::strip_generic_function_type_parameters(rest[..open].trim());
                if !is_identifier(name) {
                    return Err(
                        self.error(s, "generic functions are not supported by the debugger")
                    );
                }
                let after = rest[close + 1..].trim();
                let (_, body) = split_top_level_char(after, '=')
                    .or_else(|| after.strip_prefix('=').map(|v| ("", v)))
                    .ok_or_else(|| self.error(s, "expected function body"))?;
                let params = self.params(&rest[open + 1..close], 0)?;
                let scoped = self.scoped(params.iter().map(|p| p.name.clone()));
                result.push(Declaration::Fun(
                    name.into(),
                    params,
                    scoped.expr(body, true, 0)?,
                ));
            } else {
                if let Some(import) = s.strip_prefix("import ") {
                    if let Some((items, _)) = import.rsplit_once(" from ") {
                        for item in split_top_level(items, ',') {
                            let (name, _alias) = item
                                .trim()
                                .split_once(" as ")
                                .map_or((item.trim(), None), |(name, alias)| {
                                    (name.trim(), Some(alias))
                                });
                            if unsupported_builtin(name) {
                                return Err(self.error(s, "unsupported debug builtin import"));
                            }
                        }
                    }
                    let module = import
                        .rsplit_once(" from ")
                        .map_or(import, |(_, module)| module)
                        .trim();
                    if !module.starts_with("dw::") {
                        return Err(
                            self.error(s, "external modules are not supported by the debugger")
                        );
                    }
                }
                result.push(Declaration::Directive(s.into()));
            }
        }
        Ok(result)
    }
    fn collection_operator<'a>(&self, source: &'a str) -> Option<(&'a str, String, &'a str)> {
        crate::functions::split_collection_operator(source, &self.aliases)
    }
    fn callback(&self, source: &str, depth: usize) -> Result<Expr, DwError> {
        let s = strip_wrapping_parens(source.trim());
        if split_top_level_arrow(s).is_some() {
            self.expr(s, false, depth + 1)
        } else {
            Ok(Rc::new(Node {
                range: self.range(s),
                stop: false,
                kind: Kind::Lambda(vec![], self.expr(s, true, depth + 1)?),
            }))
        }
    }
    fn collection(
        &self,
        op: &str,
        input: &str,
        right: &str,
        depth: usize,
    ) -> Result<Kind, DwError> {
        let input = self.expr(input, false, depth + 1)?;
        if ["readLinesWith", "writeLinesWith", "scan"].contains(&op) {
            return Ok(Kind::Binary(
                op.into(),
                input,
                self.expr(right, false, depth + 1)?,
            ));
        }
        if op == "wait" {
            return Ok(Kind::Extra(super::extended::Expression::Wait(
                input,
                self.expr(right, false, depth + 1)?,
            )));
        }
        let lambda = if op == "failIf" {
            Rc::new(Node {
                range: self.range(right.trim()),
                stop: false,
                kind: Kind::Lambda(vec![], self.expr(right, true, depth + 1)?),
            })
        } else {
            self.callback(right, depth)?
        };
        Ok(Kind::Collection(op.into(), input, lambda))
    }
    fn update(&self, input: &str, right: &str, depth: usize) -> Result<Kind, DwError> {
        let right = right.trim();
        let input = self.expr(input, false, depth + 1)?;
        if right.starts_with('{')
            && find_matching_delimiter(right, 0, '{', '}') == Some(right.len() - 1)
        {
            let inner = right[1..right.len() - 1].trim();
            let sources = crate::split_update_cases(inner);
            if !inner.is_empty() && sources.first().map(|s| s.as_ptr()) != Some(inner.as_ptr()) {
                return Err(self.error(inner, "expected update case"));
            }
            let mut cases = vec![];
            for source in sources {
                let (pattern, result) = crate::split_top_level_arrow_like(source)
                    .ok_or_else(|| self.error(source, "update case requires ->"))?;
                let pattern = pattern
                    .trim()
                    .strip_prefix("case ")
                    .ok_or_else(|| self.error(pattern, "expected update case"))?
                    .trim();
                let (pattern, guard) = crate::split_update_guard(pattern);
                let (binding, path) =
                    if let Some((binding, path)) = split_top_level_keyword(pattern, "at") {
                        (binding.trim(), path.trim())
                    } else if pattern.starts_with('.') {
                        ("$", pattern)
                    } else {
                        return Err(self.error(
                            pattern,
                            "update case requires a selector or binding at selector",
                        ));
                    };
                if binding != "$" && !is_identifier(binding) {
                    return Err(self.error(pattern, "invalid update binding"));
                }
                let segments = crate::selectors::parse_path_segments(path)
                    .map_err(|_| self.error(path, "invalid update selector"))?;
                if segments.is_empty()
                    || segments.iter().any(|segment| {
                        !matches!(
                            segment,
                            crate::selectors::PathSegment::Property { .. }
                                | crate::selectors::PathSegment::Index { .. }
                        )
                    })
                {
                    return Err(
                        self.error(path, "debug updates require property or index selectors")
                    );
                }
                cases.push(UpdateCase {
                    binding: binding.into(),
                    path: path.into(),
                    guard: guard.map(|g| self.expr(g, true, depth + 1)).transpose()?,
                    body: self.expr(result, true, depth + 1)?,
                });
            }
            Ok(Kind::Update(input, Rc::new(cases)))
        } else {
            let (selector, _, replacement) =
                split_top_level_keyword_or_call_operator(right, &["with"]).ok_or_else(|| {
                    self.error(right, "update requires cases or selector with replacement")
                })?;
            Ok(Kind::UpdateWith(
                input,
                self.expr(selector, true, depth + 1)?,
                self.expr(replacement, true, depth + 1)?,
            ))
        }
    }
    fn expr(&self, source: &str, force: bool, depth: usize) -> Result<Expr, DwError> {
        if depth > 256 {
            return Err(self.error(source, "expression nesting limit exceeded"));
        }
        let s = strip_wrapping_parens(source.trim());
        if s.is_empty() {
            return Err(self.error(s, "empty expression"));
        }
        let child = |s| self.expr(s, false, depth + 1);
        let kind = if let Some(rest) = s
            .strip_prefix("do")
            .filter(|r| r.trim_start().starts_with('{'))
        {
            let rest = rest.trim();
            let end = find_matching_delimiter(rest, 0, '{', '}')
                .ok_or_else(|| self.error(s, "unterminated do block"))?;
            if end + 1 != rest.len() {
                return Err(self.error(s, "invalid do block"));
            }
            let inner = &rest[1..end];
            if let Some((a, b)) = crate::parse_script_boundary_span(inner) {
                let mut aliases = self.aliases.clone();
                for declaration in crate::functions::collect_header_declarations(&inner[..a]) {
                    if declaration.trim().starts_with("import ") {
                        crate::functions::register_import_aliases(
                            declaration.trim(),
                            &mut aliases,
                        )?;
                    }
                }
                let mut scoped = Parser {
                    source: self.source,
                    aliases,
                    shadowed: self.shadowed.clone(),
                };
                scoped.header_bindings(&inner[..a]);
                Kind::Block(
                    scoped.header(&inner[..a])?,
                    scoped.expr(&inner[b..], true, depth + 1)?,
                )
            } else {
                Kind::Block(vec![], self.expr(inner, true, depth + 1)?)
            }
        } else if s.starts_with("using ") || s.starts_with("using(") {
            self.using(s, depth)?
        } else if let Some((value, cases)) = crate::matches::parse_match_expression_source(s) {
            self.match_expression(value, cases, depth)?
        } else if let Some((condition, yes, no)) = parse_if_expression(s) {
            Kind::If(
                child(condition)?,
                self.expr(yes, true, depth + 1)?,
                self.expr(no, true, depth + 1)?,
            )
        } else if parse_literal(s)?.is_some() || self.is_whole_string(s) {
            if let Some(parts) = self.interpolation(s, depth)? {
                Kind::Extra(super::extended::Expression::Format(parts))
            } else {
                Kind::Atom(s.into())
            }
        } else if let Some(rest) = s.strip_prefix("not ") {
            Kind::Unary("not".into(), child(rest)?)
        } else if let Some(rest) = s.strip_prefix('-') {
            Kind::Unary("-".into(), child(rest)?)
        } else if let Some((a, b)) =
            split_top_level_keyword(s, "default").filter(|_| split_top_level_arrow(s).is_none())
        {
            Kind::Binary("default".into(), child(a)?, child(b)?)
        } else if let Some((a, op, b)) =
            crate::split_top_level_sequence_operator_before_arrow(s, &["++", "--"]).filter(
                |(a, _, b)| {
                    self.collection_operator(a).is_some() || self.collection_operator(b).is_some()
                },
            )
        {
            Kind::Binary(op.into(), child(a)?, child(b)?)
        } else if let Some((a, op, b)) = self.collection_operator(s) {
            self.collection(&op, a, b, depth)?
        } else if let Some((params, body)) = split_top_level_arrow(s) {
            let params = self.params(params, depth + 1)?;
            let scoped = self.scoped(params.iter().map(|p| p.name.clone()));
            Kind::Lambda(params, scoped.expr(body, true, depth + 1)?)
        } else if let Some((input, _, right)) =
            split_top_level_keyword_or_call_operator(s, &["update"])
        {
            // default short-circuits before update in the normal evaluator.
            if let Some((left, right)) = split_top_level_keyword(s, "default") {
                Kind::Binary("default".into(), child(left)?, child(right)?)
            } else {
                self.update(input, right, depth + 1)?
            }
        } else {
            if let Some((a, b, c)) = self.replace_parts(s) {
                return Ok(Rc::new(Node {
                    range: self.range(s),
                    stop: true,
                    kind: Kind::Extra(super::extended::Expression::Replace(
                        child(a)?,
                        child(b)?,
                        self.expr(c, true, depth + 1)?,
                    )),
                }));
            }
            for unsupported in ["match", "orElse", "orElseTry", "replace", "with"] {
                if s.starts_with(&format!("{unsupported} "))
                    || split_top_level_keyword(s, unsupported).is_some()
                {
                    return Err(
                        self.error(s, &format!("unsupported debug construct {unsupported}"))
                    );
                }
            }
            let mut binary = None;
            for keyword in ["default", "or", "and", "to"] {
                if let Some((a, b)) = split_top_level_keyword(s, keyword) {
                    binary = Some((a, keyword, b));
                    break;
                }
            }
            if binary.is_none() {
                binary = split_top_level_keyword_operator(s, &["mod", "pow"]);
            }
            if binary.is_none() {
                binary = split_top_level_operator(s, &["==", "!=", ">=", "<=", ">", "<"]);
            }
            if binary.is_none() {
                binary = split_top_level_operator(s, &["++", "--", "+", "-"]);
            }
            if binary.is_none() {
                binary = split_top_level_operator(s, &["*", "/"]);
            }
            if binary.is_none() {
                binary =
                    split_top_level_keyword_or_call_operator(s, crate::BINARY_BUILTIN_OPERATORS);
            }
            if let Some((a, op, b)) = binary {
                Kind::Binary(op.into(), child(a)?, child(b)?)
            } else if let Some((a, b)) = split_top_level_keyword(s, "as") {
                {
                    Kind::Type("as".into(), child(a)?, b.into())
                }
            } else if let Some((a, b)) = split_top_level_keyword(s, "is") {
                {
                    Kind::Type("is".into(), child(a)?, b.into())
                }
            } else if let Some(rest) = s.strip_prefix("not ").or_else(|| s.strip_prefix('!')) {
                Kind::Unary("not".into(), child(rest)?)
            } else if let Some(rest) = s.strip_prefix('-') {
                Kind::Unary("-".into(), child(rest)?)
            } else if s.starts_with('[')
                && find_matching_delimiter(s, 0, '[', ']') == Some(s.len() - 1)
            {
                Kind::Array(
                    split_top_level(&s[1..s.len() - 1], ',')
                        .into_iter()
                        .filter(|v| !v.trim().is_empty())
                        .map(child)
                        .collect::<Result<_, _>>()?,
                )
            } else if s.starts_with('{')
                && find_matching_delimiter(s, 0, '{', '}') == Some(s.len() - 1)
            {
                self.object(s, depth)?
            } else if let Some((base, tail)) = crate::selectors::parse_postfix_path_access(s)
                .filter(|_| parse_index_access(s).is_none())
            {
                {
                    crate::selectors::parse_path_segments(tail)?;
                    Kind::Select(child(base)?, tail.into())
                }
            } else if let Some((base, index)) = parse_index_access(s) {
                if let Some(selector) = self.selector(base, index, depth)? {
                    selector
                } else if let Some((start, end)) = split_top_level_keyword(index, "to") {
                    Kind::Slice(child(base)?, child(start)?, child(end)?)
                } else {
                    Kind::Index(child(base)?, child(index)?)
                }
            } else if let Some((target, args)) = parse_computed_call(s) {
                Kind::Extra(super::extended::Expression::Invoke(
                    child(target)?,
                    args.into_iter().map(child).collect::<Result<_, _>>()?,
                ))
            } else if let Some((name, args)) = parse_call_args(s) {
                if !is_function_name(name) {
                    return Err(self.error(s, "unsupported call target"));
                }
                if unsupported_builtin(name) {
                    return Err(self.error(s,"source-reflection and runtime-evaluation builtins are not supported by the debugger"));
                }

                let canonical =
                    crate::functions::resolve_invoked_function_name(name, &self.aliases)
                        .unwrap_or_else(|| name.into());
                if !self.shadowed.contains(name)
                    && crate::COLLECTION_OPERATORS.contains(&canonical.as_str())
                    && args.len() == 2
                {
                    self.collection(&canonical, args[0], args[1], depth)?
                } else if !self.shadowed.contains(name)
                    && ["join", "leftJoin", "outerJoin"].contains(&canonical.as_str())
                    && args.len() == 4
                {
                    Kind::Extra(super::extended::Expression::Join(
                        canonical,
                        vec![child(args[0])?, child(args[1])?],
                        vec![
                            self.callback(args[2], depth)?,
                            self.callback(args[3], depth)?,
                        ],
                    ))
                } else {
                    Kind::Call(
                        name.into(),
                        args.into_iter().map(child).collect::<Result<_, _>>()?,
                    )
                }
            } else if let Some(index) = s.char_indices().find_map(|(i, c)| {
                ((c == '.' || (c == '?' && s[i..].starts_with("?."))) && is_top_level_index(s, i))
                    .then_some(i)
            }) {
                {
                    crate::selectors::parse_path_segments(&s[index..])?;
                    Kind::Select(child(&s[..index])?, s[index..].into())
                }
            } else if is_identifier(s) || matches!(s, "$" | "$$" | "$$$") {
                Kind::Atom(s.into())
            } else {
                return Err(self.error(s, "unsupported debug expression"));
            }
        };
        let stop = force || !matches!(kind, Kind::Atom(_) | Kind::Lambda(..));
        Ok(Rc::new(Node {
            range: self.range(s),
            stop,
            kind,
        }))
    }
}
fn is_function_name(s: &str) -> bool {
    s.split("::").all(is_identifier)
}

pub(super) fn unsupported_builtin(name: &str) -> bool {
    let name = name.rsplit("::").next().unwrap_or(name);
    [
        "try",
        "eval",
        "run",
        "evalUrl",
        "readUrl",
        "location",
        "locationString",
        "evaluateCompatibilityFlag",
        "arrayItem",
        "baseTypeOf",
        "functionParamTypes",
        "functionReturnType",
        "intersectionItems",
        "literalValueOf",
        "metadataOf",
        "nameOf",
        "unionItems",
    ]
    .contains(&name)
        || (name.starts_with("is") && name.ends_with("Type"))
}
