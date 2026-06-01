#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedScript {
    pub(crate) output_directive: Option<String>,
    pub(crate) header: String,
    pub(crate) body: String,
}

pub fn parse_script_boundary(source: &str) -> Option<usize> {
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    let mut block_comment_depth = 0usize;
    let mut delimiter_depth = 0i32;

    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if in_string.is_none()
            && block_comment_depth == 0
            && delimiter_depth == 0
            && trimmed == "---"
        {
            return Some(line_index);
        }

        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if block_comment_depth > 0 {
                if ch == '/' && chars.peek() == Some(&'*') {
                    chars.next();
                    block_comment_depth += 1;
                } else if ch == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    block_comment_depth -= 1;
                }
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
                continue;
            }

            if ch == '/' && chars.peek() == Some(&'/') {
                break;
            }
            if ch == '/' && chars.peek() == Some(&'*') {
                chars.next();
                block_comment_depth = 1;
                continue;
            }
            if ch == '"' || ch == '\'' {
                in_string = Some(ch);
                continue;
            }

            match ch {
                '{' | '[' | '(' => delimiter_depth += 1,
                '}' | ']' | ')' => delimiter_depth -= 1,
                _ => {}
            }
        }
    }
    None
}

pub(crate) fn split_script(script: &str) -> ParsedScript {
    if let Some(delimiter_line) = parse_script_boundary(script) {
        let lines = script.lines().collect::<Vec<_>>();
        let header = strip_dataweave_comments(
            &lines
                .iter()
                .take(delimiter_line)
                .copied()
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let body = strip_dataweave_comments(
            &lines
                .iter()
                .skip(delimiter_line + 1)
                .copied()
                .collect::<Vec<_>>()
                .join("\n"),
        );
        return ParsedScript {
            output_directive: output_directive_from_header(&header),
            header,
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
