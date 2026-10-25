pub(crate) fn split_top_level_keyword<'a>(
    source: &'a str,
    keyword: &str,
) -> Option<(&'a str, &'a str)> {
    for (index, _) in source.match_indices(keyword) {
        let before_ok = source[..index]
            .chars()
            .last()
            .is_none_or(|ch| ch.is_whitespace());
        let after_index = index + keyword.len();
        let after_ok = source[after_index..]
            .chars()
            .next()
            .is_none_or(|ch| ch.is_whitespace());
        if before_ok && after_ok && is_top_level_index(source, index) {
            return Some((source[..index].trim(), source[after_index..].trim()));
        }
    }
    None
}

pub(crate) fn split_top_level_keyword_operator<'a, 'b>(
    source: &'a str,
    keywords: &[&'b str],
) -> Option<(&'a str, &'b str, &'a str)> {
    split_top_level_keyword_operator_with_call(source, keywords, false)
}

pub(crate) fn split_top_level_keyword_or_call_operator<'a, 'b>(
    source: &'a str,
    keywords: &[&'b str],
) -> Option<(&'a str, &'b str, &'a str)> {
    split_top_level_keyword_operator_with_call(source, keywords, true)
}

fn split_top_level_keyword_operator_with_call<'a, 'b>(
    source: &'a str,
    keywords: &[&'b str],
    allow_call: bool,
) -> Option<(&'a str, &'b str, &'a str)> {
    let mut match_value = None;
    for keyword in keywords {
        for (index, _) in source.match_indices(keyword) {
            let before_ok = source[..index]
                .chars()
                .last()
                .is_none_or(|ch| ch.is_whitespace());
            let after_index = index + keyword.len();
            let after_ok = source[after_index..]
                .chars()
                .next()
                .is_none_or(|ch| ch.is_whitespace() || (allow_call && ch == '('));
            if before_ok
                && after_ok
                && is_top_level_index(source, index)
                && split_top_level_arrow(&source[..index]).is_none()
            {
                if match_value.is_none_or(|(_, _, _, matched_index)| index > matched_index) {
                    match_value = Some((
                        source[..index].trim(),
                        *keyword,
                        source[after_index..].trim(),
                        index,
                    ));
                }
            }
        }
    }
    match_value
        .filter(|(left, _, right, _)| !left.is_empty() && !right.is_empty())
        .map(|(left, keyword, right, _)| (left, keyword, right))
}

pub(crate) fn split_top_level_operator<'a>(
    source: &'a str,
    operators: &[&'static str],
) -> Option<(&'a str, &'static str, &'a str)> {
    let mut match_value = None;
    for (index, _) in source.char_indices() {
        if !is_top_level_index(source, index) {
            continue;
        }
        for operator in operators {
            if source[index..].starts_with(operator)
                && !(operator == &"<" && starts_type_argument_list(source, index))
                && is_binary_operator_position(source, index)
            {
                match_value = Some((
                    source[..index].trim(),
                    *operator,
                    source[index + operator.len()..].trim(),
                ));
                break;
            }
        }
    }
    match_value.filter(|(left, _, right)| !left.is_empty() && !right.is_empty())
}

pub(crate) fn split_top_level_arrow(source: &str) -> Option<(&str, &str)> {
    source
        .match_indices("->")
        .find(|(index, _)| is_top_level_index(source, *index))
        .map(|(index, _)| (&source[..index], &source[index + 2..]))
}

pub(crate) fn is_binary_operator_position(source: &str, index: usize) -> bool {
    let Some(previous) = source[..index].chars().rev().find(|ch| !ch.is_whitespace()) else {
        return false;
    };
    !matches!(
        previous,
        '(' | '[' | '{' | ':' | ',' | '.' | '+' | '-' | '*' | '/' | '<' | '>' | '=' | '!'
    )
}

pub(crate) fn parse_if_expression(source: &str) -> Option<(&str, &str, &str)> {
    let rest = source.strip_prefix("if")?.trim_start();
    if !rest.starts_with('(') {
        return None;
    }
    let condition_end = find_matching_delimiter(rest, 0, '(', ')')?;
    let condition = &rest[1..condition_end];
    let after_condition = rest[condition_end + 1..].trim();
    let (when_true, when_false) = split_top_level_keyword(after_condition, "else")?;
    Some((condition.trim(), when_true.trim(), when_false.trim()))
}

pub(crate) fn parse_index_access(source: &str) -> Option<(&str, &str)> {
    if !source.ends_with(']') {
        return None;
    }
    for (index, ch) in source.char_indices().rev() {
        if ch == '[' && is_top_level_index(source, index) {
            return Some((
                source[..index].trim(),
                source[index + 1..source.len() - 1].trim(),
            ));
        }
    }
    None
}

pub(crate) fn find_matching_delimiter(
    source: &str,
    open_index: usize,
    open: char,
    close: char,
) -> Option<usize> {
    if source.as_bytes().get(open_index).copied() != Some(open as u8) {
        return None;
    }
    matching_end(source, open_index, close as u8, false)
}

/// Tokens for tolerant editor analysis. Uses the runtime literal/comment scanner;
/// an unfinished token keeps all preceding declarations available.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CodeToken<'a> {
    pub text: &'a str,
    pub start: usize,
    pub end: usize,
    pub literal: bool,
}

pub(crate) fn editor_tokens(source: &str) -> Vec<CodeToken<'_>> {
    fn scan<'a>(
        source: &'a str,
        start: usize,
        limit: usize,
        out: &mut Vec<CodeToken<'a>>,
        depth: usize,
    ) {
        if depth > 128 {
            return;
        }
        let bytes = source.as_bytes();
        let mut i = start;
        while i < limit {
            let c = source[i..].chars().next().unwrap();
            if c.is_whitespace() {
                i += c.len_utf8();
                continue;
            }
            if starts_line_comment(source, i) {
                i = source[i..limit].find('\n').map_or(limit, |n| i + n);
                continue;
            }
            if bytes[i..].starts_with(b"/*") {
                i = comment_end(source, i).unwrap_or(limit).min(limit);
                continue;
            }
            if matches!(c, '"' | '\'' | '`')
                || (c == '/' && starts_regex_literal(source, i))
                || (c == '|' && starts_temporal_literal(source, i))
            {
                let end = matching_end(source, i, c as u8, true)
                    .map_or(limit, |n| n + 1)
                    .min(limit);
                out.push(CodeToken {
                    text: &source[i..end],
                    start: i,
                    end,
                    literal: true,
                });
                if matches!(c, '"' | '\'' | '`') {
                    let mut p = i + 1;
                    while p + 1 < end {
                        if bytes[p] == b'\\' {
                            p += 2;
                            continue;
                        }
                        if bytes[p..].starts_with(b"$(") {
                            let open = p + 1;
                            let close = find_matching_delimiter(source, open, '(', ')')
                                .filter(|n| *n < end);
                            out.push(CodeToken {
                                text: "(",
                                start: open,
                                end: open + 1,
                                literal: false,
                            });
                            scan(source, open + 1, close.unwrap_or(end), out, depth + 1);
                            if let Some(close) = close {
                                out.push(CodeToken {
                                    text: ")",
                                    start: close,
                                    end: close + 1,
                                    literal: false,
                                });
                                p = close + 1;
                            } else {
                                break;
                            }
                        } else {
                            p += 1;
                        }
                    }
                }
                i = end;
                continue;
            }
            let start = i;
            if c.is_ascii_alphabetic() || matches!(c, '_' | '$') {
                i += 1;
                while i < limit
                    && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'$'))
                {
                    i += 1;
                }
            } else {
                let width = if source[i..].starts_with("---") {
                    3
                } else if source[i..].starts_with("->") || source[i..].starts_with("::") {
                    2
                } else {
                    c.len_utf8()
                };
                i += width;
            }
            out.push(CodeToken {
                text: &source[start..i],
                start,
                end: i,
                literal: false,
            });
        }
    }
    let mut out = vec![];
    scan(source, 0, source.len(), &mut out, 0);
    out
}
fn matching_end(source: &str, start: usize, close: u8, quote: bool) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut frames = vec![(close, quote)];
    let mut i = start + 1;
    while i < bytes.len() {
        let c = bytes[i];
        let (end, literal) = *frames.last()?;
        if literal {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == end {
                frames.pop();
                if frames.is_empty() {
                    return Some(i);
                }
                i += 1;
                continue;
            }
            if matches!(end, b'"' | b'\'' | b'`') && bytes[i..].starts_with(b"$(") {
                frames.push((b')', false));
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if starts_line_comment(source, i) {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i..].starts_with(b"/*") {
            i = comment_end(source, i)?;
            continue;
        }
        if matches!(c, b'"' | b'\'' | b'`')
            || (c == b'/' && starts_regex_literal(source, i))
            || (c == b'|' && starts_temporal_literal(source, i))
        {
            frames.push((c, true));
            i += 1;
            continue;
        }
        if c == end {
            frames.pop();
            if frames.is_empty() {
                return Some(i);
            }
            i += 1;
            continue;
        }
        match c {
            b'(' => frames.push((b')', false)),
            b'[' => frames.push((b']', false)),
            b'{' => frames.push((b'}', false)),
            _ => (),
        }
        if frames.len() > 10000 {
            return None;
        }
        i += 1;
    }
    None
}
fn comment_end(source: &str, start: usize) -> Option<usize> {
    let b = source.as_bytes();
    let mut i = start + 2;
    let mut depth = 1;
    while i < b.len() {
        if b[i..].starts_with(b"/*") {
            depth += 1;
            i += 2;
        } else if b[i..].starts_with(b"*/") {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return Some(i);
            }
        } else {
            i += 1;
        }
    }
    None
}

pub(crate) fn split_top_level_char(source: &str, delimiter: char) -> Option<(&str, &str)> {
    source
        .char_indices()
        .find(|(index, ch)| {
            *ch == delimiter
                && !is_qualified_separator(source, *index, delimiter)
                && is_top_level_index(source, *index)
        })
        .map(|(index, _)| (&source[..index], &source[index + delimiter.len_utf8()..]))
}

fn is_qualified_separator(source: &str, index: usize, delimiter: char) -> bool {
    delimiter == ':'
        && (source[..index].ends_with(':')
            || source[index + delimiter.len_utf8()..].starts_with(':'))
}

pub(crate) fn split_top_level(source: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    for (index, ch) in source.char_indices() {
        if ch == delimiter && is_top_level_index(source, index) {
            parts.push(&source[start..index]);
            start = index + ch.len_utf8();
        }
    }
    parts.push(&source[start..]);
    parts
}

pub(crate) fn is_top_level_index(source: &str, target: usize) -> bool {
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut depth = 0i32;
    let mut angle = 0;
    while i < target && i < bytes.len() {
        let c = bytes[i];
        if starts_line_comment(source, i) {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            if i >= target {
                return false;
            }
            continue;
        }
        if bytes[i..].starts_with(b"/*") {
            let Some(end) = comment_end(source, i) else {
                return false;
            };
            if end > target {
                return false;
            }
            i = end;
            continue;
        }
        if matches!(c, b'"' | b'\'' | b'`')
            || (c == b'/' && starts_regex_literal(source, i))
            || (c == b'|' && starts_temporal_literal(source, i))
        {
            let Some(end) = matching_end(source, i, c, true) else {
                return false;
            };
            if end >= target {
                return false;
            }
            i = end + 1;
            continue;
        }
        match c {
            b'<' if starts_type_argument_list(source, i) => angle += 1,
            b'>' if angle > 0 => angle -= 1,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => (),
        };
        i += 1;
    }
    depth == 0 && angle == 0
}

pub(crate) fn starts_temporal_literal(source: &str, index: usize) -> bool {
    let Some(rest) = source.get(index..).and_then(|s| s.strip_prefix('|')) else {
        return false;
    };
    let mut chars = rest.chars();
    match chars.next() {
        Some(c) if c.is_ascii_digit() => true,
        Some('P') => chars.next().is_some_and(|c| c.is_ascii_digit() || c == 'T'),
        Some('T') => chars.next().is_some_and(|c| c.is_ascii_digit()),
        Some('+' | '-') => chars.next().is_some_and(|c| c.is_ascii_digit() || c == 'P'),
        Some('Z') => chars.next() == Some('|'),
        _ => false,
    }
}

pub(crate) fn starts_regex_literal(source: &str, index: usize) -> bool {
    if !source[index..].starts_with('/')
        || !source[index + 1..]
            .chars()
            .next()
            .is_some_and(|next| next != '/' && next != '*')
    {
        return false;
    }
    let before = source[..index].trim_end();
    before
        .chars()
        .last()
        .is_none_or(|ch| matches!(ch, '(' | '[' | '{' | ':' | ',' | '='))
        || before
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
            .next_back()
            .is_some_and(|word| matches!(word, "matches" | "replace" | "with" | "case"))
}

fn starts_type_argument_list(source: &str, index: usize) -> bool {
    if !source[index..].starts_with('<')
        || !source[index + 1..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !source[..index]
            .chars()
            .rev()
            .find(|c| !c.is_whitespace())
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '>')
    {
        return false;
    }
    let bytes = source.as_bytes();
    let mut i = index + 1;
    let mut angles = 1;
    let mut groups = vec![];
    while i < bytes.len() {
        let c = bytes[i];
        if matches!(c, b'"' | b'\'' | b'`') {
            let Some(end) = matching_end(source, i, c, true) else {
                return false;
            };
            i = end + 1;
            continue;
        }
        if bytes[i..].starts_with(b"->") {
            i += 2;
            continue;
        }
        match c {
            b'<' => angles += 1,
            b'>' => {
                angles -= 1;
                if angles == 0 {
                    let inside = &source[index + 1..i];
                    if inside
                        .split_whitespace()
                        .any(|w| matches!(w, "and" | "or" | "default" | "else" | "if"))
                    {
                        return false;
                    }
                    return !source[i + 1..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
                }
            }
            b'(' => groups.push(b')'),
            b'[' => groups.push(b']'),
            b'{' => groups.push(b'}'),
            b')' | b']' | b'}' => {
                if groups.pop() != Some(c) {
                    return false;
                }
            }
            b';' | b'=' => return false,
            _ => (),
        }
        i += 1;
    }
    false
}

pub(crate) fn strip_wrapping_parens(source: &str) -> &str {
    let mut current = source;
    while current.starts_with('(') && current.ends_with(')') && matching_outer_parens(current) {
        current = current[1..current.len() - 1].trim();
    }
    current
}

pub(crate) fn matching_outer_parens(source: &str) -> bool {
    find_matching_delimiter(source, 0, '(', ')') == Some(source.len() - 1)
}

pub(crate) fn is_identifier(source: &str) -> bool {
    let mut chars = source.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// A call whose target is an expression, rather than a qualified function name.
/// Returned slices remain borrowed from the original source.
pub(crate) fn parse_computed_call(source: &str) -> Option<(&str, Vec<&str>)> {
    if !source.ends_with(')') {
        return None;
    }
    for (open, ch) in source.char_indices() {
        if ch != '(' || open == 0 || !is_top_level_index(source, open) {
            continue;
        }
        if find_matching_delimiter(source, open, '(', ')') != Some(source.len() - 1) {
            continue;
        }
        let target = source[..open].trim();
        let name = target.split('<').next().unwrap_or(target);
        if name.split("::").all(is_identifier) && (name == target || target.ends_with('>')) {
            return None;
        }
        let inner = source[open + 1..source.len() - 1].trim();
        let args = if inner.is_empty() {
            vec![]
        } else {
            split_top_level(inner, ',')
                .into_iter()
                .map(str::trim)
                .collect()
        };
        return Some((target, args));
    }
    None
}

/// Visit code positions at delimiter depth zero, skipping complete literals,
/// including interpolated strings, in one pass. Shared by source-aware grammars.
pub(crate) fn visit_top_level(source: &str, mut visit: impl FnMut(usize, char)) {
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut depth = 0i32;
    let mut angle = 0;
    while i < bytes.len() {
        if starts_line_comment(source, i) {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i..].starts_with(b"/*") {
            let Some(end) = comment_end(source, i) else {
                return;
            };
            i = end;
            continue;
        }
        let ch = source[i..].chars().next().unwrap();
        let root = depth == 0 && angle == 0;
        if matches!(ch, '"' | '\'' | '`')
            || (ch == '/' && starts_regex_literal(source, i))
            || (ch == '|' && starts_temporal_literal(source, i))
        {
            if root {
                visit(i, ch);
            }
            let Some(end) = matching_end(source, i, ch as u8, true) else {
                return;
            };
            i = end + 1;
            continue;
        }
        if root {
            visit(i, ch);
        }
        match ch {
            '<' if starts_type_argument_list(source, i) => angle += 1,
            '>' if angle > 0 => angle -= 1,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => (),
        };
        i += ch.len_utf8();
    }
}

/// Replace comments byte-for-byte, preserving newlines and UTF-8 offsets.
pub(crate) fn mask_comments(source: &str) -> Result<String, crate::DwError> {
    enum Mode {
        Code(usize),
        Literal(u8),
    }
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut modes = vec![Mode::Code(0)];
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let next = bytes.get(i + 1).copied();
        if let Some(Mode::Literal(quote)) = modes.last() {
            let quote = *quote;
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == quote {
                modes.pop();
                i += 1;
                continue;
            }
            if matches!(quote, b'"' | b'\'' | b'`') && c == b'$' && next == Some(b'(') {
                modes.push(Mode::Code(1));
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if c == b'/' && next == Some(b'/') && starts_line_comment(source, i) {
            while i < bytes.len() && bytes[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
            continue;
        }
        if c == b'/' && next == Some(b'*') {
            let mut depth = 1;
            out[i] = b' ';
            out[i + 1] = b' ';
            i += 2;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                } else {
                    if !matches!(bytes[i], b'\n' | b'\r') {
                        out[i] = b' ';
                    }
                    i += 1;
                }
            }
            if depth > 0 {
                return Err(crate::DwError::Parse("Debug: unterminated comment".into()));
            }
            continue;
        }
        if matches!(c, b'"' | b'\'' | b'`')
            || (c == b'|' && starts_temporal_literal(source, i))
            || (c == b'/' && starts_regex_literal(source, i))
        {
            modes.push(Mode::Literal(c));
            i += 1;
            continue;
        }
        if let Some(Mode::Code(depth)) = modes.last_mut() {
            if *depth > 0 {
                if c == b'(' {
                    *depth += 1;
                } else if c == b')' {
                    *depth -= 1;
                    if *depth == 0 {
                        modes.pop();
                    }
                }
            }
        }
        i += 1;
    }
    if modes.len() != 1 {
        return Err(crate::DwError::Parse(
            "Debug: unterminated comment or literal".into(),
        ));
    }
    Ok(String::from_utf8(out).expect("masked source is UTF-8"))
}

fn starts_line_comment(source: &str, index: usize) -> bool {
    source.as_bytes()[index..].starts_with(b"//")
        && source[..index].chars().rev().find(|c| !c.is_whitespace()) != Some(':')
}
