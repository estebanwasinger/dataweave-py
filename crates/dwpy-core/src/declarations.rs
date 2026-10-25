/// Declaration ranges from the shared lexer, also used by editor services.
pub(crate) fn header_declaration_spans(header: &str) -> Vec<std::ops::Range<usize>> {
    let mut candidates = vec![];
    let mut offset = 0;
    for line in header.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if is_header_declaration_start(trimmed) {
            candidates.push(offset + line.len() - trimmed.len());
        }
        offset += line.len();
    }
    let mut starts = vec![];
    crate::syntax::visit_top_level(header, |index, _| {
        if candidates.binary_search(&index).is_ok() {
            starts.push(index);
        }
    });
    if starts.is_empty() {
        return if header.trim().is_empty() {
            vec![]
        } else {
            vec![0..header.len()]
        };
    }
    let mut out = vec![];
    if !header[..starts[0]].trim().is_empty() {
        out.push(0..starts[0]);
    }
    starts.push(header.len());
    for pair in starts.windows(2) {
        out.push(pair[0]..pair[1]);
    }
    out
}

fn is_header_declaration_start(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("%dw")
        || trimmed.starts_with("output ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("input ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("ns ")
        || trimmed.starts_with("var ")
        || trimmed.starts_with("fun ")
}

pub(crate) fn strip_generic_function_type_parameters(source: &str) -> &str {
    let Some(open) = source.find('<') else {
        return source;
    };
    let Some(close) = source.rfind('>') else {
        return source;
    };
    if close == source.len() - 1 {
        source[..open].trim()
    } else {
        source
    }
}
