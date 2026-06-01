#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedScript {
    pub(crate) output_directive: Option<String>,
    pub(crate) header: String,
    pub(crate) body: String,
}

pub fn parse_script_boundary(source: &str) -> Option<usize> {
    parse_script_boundary_span(source).map(|(start, _)| source[..start].lines().count())
}

pub fn parse_script_boundary_span(source: &str) -> Option<(usize, usize)> {
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    let mut block_comment_depth = 0usize;
    let mut delimiter_depth = 0i32;

    let bytes = source.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        let next = bytes.get(index + 1).copied().map(char::from);

        if block_comment_depth > 0 {
            if ch == '/' && next == Some('*') {
                block_comment_depth += 1;
                index += 2;
                continue;
            }
            if ch == '*' && next == Some('/') {
                block_comment_depth -= 1;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(quote) = in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            index += 1;
            continue;
        }

        if ch == '/' && next == Some('/') {
            let Some(newline_offset) = source[index..].find('\n') else {
                break;
            };
            index += newline_offset + 1;
            continue;
        }
        if ch == '/' && next == Some('*') {
            block_comment_depth = 1;
            index += 2;
            continue;
        }

        if bytes[index..].starts_with(b"---")
            && delimiter_depth == 0
            && has_delimiter_boundary(source, index)
        {
            return Some((index, index + 3));
        }

        if ch == '"' || ch == '\'' || ch == '`' {
            in_string = Some(ch);
            index += 1;
            continue;
        }

        match ch {
            '{' | '[' | '(' => delimiter_depth += 1,
            '}' | ']' | ')' => delimiter_depth -= 1,
            _ => {}
        }
        index += 1;
    }
    None
}

pub(crate) fn split_script(script: &str) -> ParsedScript {
    if let Some((delimiter_start, delimiter_end)) = parse_script_boundary_span(script) {
        let header = strip_dataweave_comments(&script[..delimiter_start]);
        let body = strip_dataweave_comments(&script[delimiter_end..]);
        return ParsedScript {
            output_directive: output_directive_from_header(&header),
            header: header.trim().to_string(),
            body: body.trim().to_string(),
        };
    }

    let body = strip_dataweave_comments(script);
    ParsedScript {
        output_directive: output_directive_from_header(&body),
        header: String::new(),
        body: body.trim().to_string(),
    }
}

fn has_delimiter_boundary(source: &str, index: usize) -> bool {
    let before_ok = index == 0
        || source[..index]
            .chars()
            .next_back()
            .is_none_or(char::is_whitespace);
    let after_index = index + 3;
    let after_ok = after_index == source.len()
        || source[after_index..]
            .chars()
            .next()
            .is_none_or(char::is_whitespace);
    before_ok && after_ok
}

fn output_directive_from_header(header: &str) -> Option<String> {
    header
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("output ").map(str::trim))
        .map(str::to_string)
}

fn strip_dataweave_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    let mut block_depth = 0usize;
    let mut previous_significant: Option<char> = None;

    while let Some(ch) = chars.next() {
        if block_depth > 0 {
            if ch == '/' && chars.peek() == Some(&'*') {
                chars.next();
                block_depth += 1;
                output.push(' ');
                output.push(' ');
            } else if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_depth -= 1;
                output.push(' ');
                output.push(' ');
            } else if ch == '\n' {
                output.push('\n');
            } else {
                output.push(' ');
            }
            continue;
        }

        if let Some(quote) = in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
                previous_significant = Some(quote);
            }
            continue;
        }

        if ch == '/'
            && chars
                .peek()
                .is_some_and(|next| *next != '/' && *next != '*')
            && previous_significant
                .is_none_or(|prev| matches!(prev, '(' | '[' | '{' | ':' | ',' | '='))
        {
            in_string = Some('/');
            output.push(ch);
            previous_significant = Some(ch);
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') && previous_significant != Some(':') {
            chars.next();
            output.push(' ');
            output.push(' ');
            for comment_ch in chars.by_ref() {
                if comment_ch == '\n' {
                    output.push('\n');
                    break;
                }
                output.push(' ');
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_depth = 1;
            output.push(' ');
            output.push(' ');
            continue;
        }

        if ch == '"' || ch == '\'' || ch == '`' {
            in_string = Some(ch);
        }
        output.push(ch);
        if !ch.is_whitespace() {
            previous_significant = Some(ch);
        }
    }

    output
}
