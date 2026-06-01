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

pub(crate) fn split_top_level_keyword_operator<'a>(
    source: &'a str,
    keywords: &[&'static str],
) -> Option<(&'a str, &'static str, &'a str)> {
    split_top_level_keyword_operator_with_call(source, keywords, false)
}

pub(crate) fn split_top_level_keyword_or_call_operator<'a>(
    source: &'a str,
    keywords: &[&'static str],
) -> Option<(&'a str, &'static str, &'a str)> {
    split_top_level_keyword_operator_with_call(source, keywords, true)
}

fn split_top_level_keyword_operator_with_call<'a>(
    source: &'a str,
    keywords: &[&'static str],
    allow_call: bool,
) -> Option<(&'a str, &'static str, &'a str)> {
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
    let mut depth = 0i32;
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    for (index, ch) in source
        .char_indices()
        .skip_while(|(index, _)| *index < open_index)
    {
        if let Some(quote) = in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            continue;
        }
        if ch == '/' && starts_regex_literal(source, index) {
            in_string = Some('/');
            continue;
        }
        if ch == '"'
            || ch == '\''
            || ch == '`'
            || (ch == '|' && starts_temporal_literal(source, index))
        {
            in_string = Some(ch);
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
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
    let mut depth = 0i32;
    let mut angle_depth = 0i32;
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    for (index, ch) in source.char_indices() {
        if index >= target {
            return depth == 0 && angle_depth == 0 && in_string.is_none();
        }
        if let Some(quote) = in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            continue;
        }
        match ch {
            '/' if starts_regex_literal(source, index) => in_string = Some('/'),
            '"' | '\'' | '`' => in_string = Some(ch),
            '|' if starts_temporal_literal(source, index) => in_string = Some(ch),
            '<' if starts_type_argument_list(source, index) => angle_depth += 1,
            '>' if angle_depth > 0 => angle_depth -= 1,
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            _ => {}
        }
    }
    depth == 0 && angle_depth == 0 && in_string.is_none()
}

fn starts_temporal_literal(source: &str, index: usize) -> bool {
    source[index..].starts_with('|')
        && source[index + '|'.len_utf8()..]
            .chars()
            .next()
            .is_some_and(|next| next.is_ascii_digit() || matches!(next, 'P' | 'T' | '+' | '-'))
}

fn starts_regex_literal(source: &str, index: usize) -> bool {
    source[index..].starts_with('/')
        && source[index + '/'.len_utf8()..]
            .chars()
            .next()
            .is_some_and(|next| next != '/' && next != '*')
        && source[..index]
            .chars()
            .rev()
            .find(|ch| !ch.is_whitespace())
            .is_none_or(|ch| matches!(ch, '(' | '[' | '{' | ':' | ',' | '='))
}

fn starts_type_argument_list(source: &str, index: usize) -> bool {
    source[index..].starts_with('<')
        && source[index + '<'.len_utf8()..]
            .chars()
            .next()
            .is_some_and(|next| next.is_ascii_alphabetic() || next == '_')
        && source[..index]
            .chars()
            .rev()
            .find(|ch| !ch.is_whitespace())
            .is_some_and(|previous| {
                previous.is_ascii_alphabetic() || previous == '_' || previous == '>'
            })
}

pub(crate) fn strip_wrapping_parens(source: &str) -> &str {
    let mut current = source;
    while current.starts_with('(') && current.ends_with(')') && matching_outer_parens(current) {
        current = current[1..current.len() - 1].trim();
    }
    current
}

pub(crate) fn matching_outer_parens(source: &str) -> bool {
    let mut depth = 0i32;
    for (index, ch) in source.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && index != source.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

pub(crate) fn is_identifier(source: &str) -> bool {
    let mut chars = source.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
