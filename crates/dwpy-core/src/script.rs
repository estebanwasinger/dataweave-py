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
    let mut boundary = None;
    crate::syntax::visit_top_level(source, |index, ch| {
        if boundary.is_none()
            && ch == '-'
            && source[index..].starts_with("---")
            && has_delimiter_boundary(source, index)
        {
            boundary = Some((index, index + 3));
        }
    });
    boundary
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

pub(crate) fn strip_dataweave_comments(source: &str) -> String {
    crate::syntax::mask_comments(source).unwrap_or_else(|_| source.to_owned())
}
