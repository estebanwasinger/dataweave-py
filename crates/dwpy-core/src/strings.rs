use serde_json::Value;

use regex::Regex;

use crate::{as_dataweave_string, char_slice, numeric_value, DwError};

pub(crate) fn append_if_missing(text: &Value, suffix: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let suffix = if suffix.is_null() {
        String::new()
    } else {
        as_dataweave_string(suffix)
    };
    if source.ends_with(&suffix) {
        Ok(Value::String(source))
    } else {
        Ok(Value::String(format!("{source}{suffix}")))
    }
}

pub(crate) fn prepend_if_missing(text: &Value, prefix: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let prefix = if prefix.is_null() {
        String::new()
    } else {
        as_dataweave_string(prefix)
    };
    if source.starts_with(&prefix) {
        Ok(Value::String(source))
    } else {
        Ok(Value::String(format!("{prefix}{source}")))
    }
}

pub(crate) fn camelize(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let source = split_camel_with_separator(&as_dataweave_string(value), '_');
    let parts: Vec<String> = source
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| part.to_lowercase())
        .collect();
    if parts.is_empty() {
        return Ok(Value::String(String::new()));
    }
    let mut output = parts[0].clone();
    for part in parts.iter().skip(1) {
        output.push_str(&capitalize_word(part));
    }
    Ok(Value::String(output))
}

pub(crate) fn capitalize(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let separated = split_camel_with_separator(&as_dataweave_string(value), ' ').replace('_', " ");
    let words: Vec<String> = separated
        .split_whitespace()
        .map(|word| capitalize_word(&word.to_lowercase()))
        .collect();
    Ok(Value::String(words.join(" ")))
}

fn capitalize_word(word: &str) -> String {
    let mut output = String::new();
    let mut capitalize_next = true;
    for ch in word.chars() {
        if capitalize_next {
            output.extend(ch.to_uppercase());
        } else {
            output.extend(ch.to_lowercase());
        }
        capitalize_next = !ch.is_alphanumeric();
    }
    output
}

pub(crate) fn char_code(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(value);
    let Some(first) = source.chars().next() else {
        return Err(DwError::UnsupportedFeature(
            "charCode expects a non-empty string".to_string(),
        ));
    };
    Ok(Value::Number((first as u32 as i64).into()))
}

pub(crate) fn char_code_at(content: &Value, position: &Value) -> Result<Value, DwError> {
    if content.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(content);
    let index = numeric_value(position)? as i64;
    if index < 0 {
        return Err(DwError::UnsupportedFeature(
            "charCodeAt index out of range".to_string(),
        ));
    }
    let Some(ch) = source.chars().nth(index as usize) else {
        return Err(DwError::UnsupportedFeature(
            "charCodeAt index out of range".to_string(),
        ));
    };
    Ok(Value::Number((ch as u32 as i64).into()))
}

pub(crate) fn collapse_string(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(value);
    let mut groups: Vec<Value> = Vec::new();
    let mut chars = source.chars();
    let Some(mut current) = chars.next() else {
        return Ok(Value::Array(groups));
    };
    let mut run = current.to_string();
    for ch in chars {
        if ch == current {
            run.push(ch);
        } else {
            groups.push(Value::String(run));
            current = ch;
            run = ch.to_string();
        }
    }
    groups.push(Value::String(run));
    Ok(Value::Array(groups))
}

pub(crate) fn count_matches(text: &Value, pattern: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    if pattern.is_null() {
        return Ok(Value::Number(0.into()));
    }
    let source = as_dataweave_string(text);
    let pattern = as_dataweave_string(pattern);
    if let Some(pattern) = regex_literal_inner(&pattern) {
        let regex = Regex::new(pattern).map_err(|err| {
            DwError::UnsupportedFeature(format!("regex pattern /{pattern}/: {err}"))
        })?;
        return Ok(Value::Number(regex.find_iter(&source).count().into()));
    }
    if pattern.is_empty() {
        return Ok(Value::Number(0.into()));
    }
    let mut count = 0i64;
    let mut cursor = 0usize;
    while let Some(index) = source[cursor..].find(&pattern) {
        count += 1;
        cursor += index + pattern.len();
        if cursor > source.len() {
            break;
        }
    }
    Ok(Value::Number(count.into()))
}

fn split_camel_with_separator(source: &str, separator: char) -> String {
    let mut output = String::new();
    let mut previous: Option<char> = None;
    for ch in source.chars() {
        if ch.is_uppercase()
            && previous
                .map(|prev| prev.is_lowercase() || prev.is_ascii_digit())
                .unwrap_or(false)
        {
            output.push(separator);
        }
        output.push(ch);
        previous = Some(ch);
    }
    output
}

fn normalize_separators(
    source: &str,
    separator: char,
    separator_predicate: fn(char) -> bool,
) -> String {
    let mut output = String::new();
    let mut last_was_separator = false;
    for ch in source.chars() {
        if separator_predicate(ch) {
            if !last_was_separator {
                output.push(separator);
                last_was_separator = true;
            }
        } else {
            output.push(ch);
            last_was_separator = false;
        }
    }
    output.trim_matches(separator).to_string()
}

pub(crate) fn dasherize(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let separated = split_camel_with_separator(&as_dataweave_string(value), '-');
    Ok(Value::String(
        normalize_separators(&separated, '-', |ch| ch == '_' || ch.is_whitespace()).to_lowercase(),
    ))
}

pub(crate) fn underscore(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let separated = split_camel_with_separator(&as_dataweave_string(value), '_');
    Ok(Value::String(
        normalize_separators(&separated, '_', |ch| ch == '-' || ch.is_whitespace()).to_lowercase(),
    ))
}

pub(crate) fn first_string(text: &Value, amount: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let size = numeric_value(amount)?.floor() as i64;
    if size <= 0 {
        return Ok(Value::String(String::new()));
    }
    let len = source.chars().count();
    Ok(Value::String(char_slice(
        &source,
        0,
        (size as usize).min(len),
    )))
}

pub(crate) fn last_string(text: &Value, amount: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let size = numeric_value(amount)?.ceil() as i64;
    if size <= 0 {
        return Ok(Value::String(String::new()));
    }
    let len = source.chars().count();
    if size as usize >= len {
        return Ok(Value::String(source));
    }
    Ok(Value::String(char_slice(&source, len - size as usize, len)))
}

pub(crate) fn from_char_code(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let code = numeric_value(value)? as u32;
    let Some(ch) = char::from_u32(code) else {
        return Err(DwError::UnsupportedFeature(format!(
            "invalid char code {code}"
        )));
    };
    Ok(Value::String(ch.to_string()))
}

pub(crate) fn hamming_distance(left: &Value, right: &Value) -> Result<Value, DwError> {
    if left.is_null() || right.is_null() {
        return Ok(Value::Null);
    }
    let left_chars: Vec<char> = as_dataweave_string(left).chars().collect();
    let right_chars: Vec<char> = as_dataweave_string(right).chars().collect();
    if left_chars.len() != right_chars.len() {
        return Ok(Value::Null);
    }
    Ok(Value::Number(
        left_chars
            .iter()
            .zip(right_chars.iter())
            .filter(|(left, right)| left != right)
            .count()
            .into(),
    ))
}

pub(crate) fn is_alpha(value: &Value) -> bool {
    if value.is_null() {
        return false;
    }
    let source = as_dataweave_string(value);
    !source.is_empty() && source.chars().all(|ch| ch.is_alphabetic())
}

pub(crate) fn is_alphanumeric(value: &Value) -> bool {
    if value.is_null() {
        return false;
    }
    let source = as_dataweave_string(value);
    !source.is_empty() && source.chars().all(|ch| ch.is_alphanumeric())
}

pub(crate) fn is_lower_case(value: &Value) -> bool {
    if value.is_null() {
        return false;
    }
    let source = as_dataweave_string(value);
    !source.is_empty()
        && source
            .chars()
            .all(|ch| ch.is_alphabetic() && ch.is_lowercase())
}

pub(crate) fn is_upper_case(value: &Value) -> bool {
    if value.is_null() {
        return false;
    }
    let source = as_dataweave_string(value);
    !source.is_empty()
        && source
            .chars()
            .all(|ch| ch.is_alphabetic() && ch.is_uppercase())
}

pub(crate) fn is_whitespace(value: &Value) -> bool {
    if value.is_null() {
        return false;
    }
    as_dataweave_string(value)
        .chars()
        .all(|ch| ch.is_whitespace())
}

pub(crate) fn pad_string(
    text: &Value,
    size: &Value,
    pad: &Value,
    left_pad: bool,
) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let target_size = numeric_value(size)? as i64;
    let source_len = source.chars().count() as i64;
    if target_size <= source_len {
        return Ok(Value::String(source));
    }
    let pad_text = if pad.is_null() {
        " ".to_string()
    } else {
        as_dataweave_string(pad)
    };
    if pad_text.is_empty() {
        return Ok(Value::String(source));
    }
    let padding = build_padding(&pad_text, (target_size - source_len) as usize);
    if left_pad {
        Ok(Value::String(format!("{padding}{source}")))
    } else {
        Ok(Value::String(format!("{source}{padding}")))
    }
}

fn build_padding(pad_text: &str, length: usize) -> String {
    pad_text.chars().cycle().take(length).collect()
}

pub(crate) fn levenshtein_distance(left: &Value, right: &Value) -> Result<Value, DwError> {
    if left.is_null() || right.is_null() {
        return Ok(Value::Null);
    }
    let left: Vec<char> = as_dataweave_string(left).chars().collect();
    let right: Vec<char> = as_dataweave_string(right).chars().collect();
    if left == right {
        return Ok(Value::Number(0.into()));
    }
    if left.is_empty() {
        return Ok(Value::Number(right.len().into()));
    }
    if right.is_empty() {
        return Ok(Value::Number(left.len().into()));
    }
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (left_index, left_char) in left.iter().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_char) in right.iter().enumerate() {
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            let replacement = previous[right_index] + usize::from(left_char != right_char);
            current.push(insertion.min(deletion).min(replacement));
        }
        previous = current;
    }
    Ok(Value::Number(previous[right.len()].into()))
}

pub(crate) fn lines(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    Ok(Value::Array(
        as_dataweave_string(value)
            .lines()
            .map(|line| Value::String(line.to_string()))
            .collect(),
    ))
}

pub(crate) fn ordinalize(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let number = numeric_value(value)? as i64;
    let absolute = number.abs();
    let suffix = if (10..=20).contains(&(absolute % 100)) {
        "th"
    } else {
        match absolute % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    Ok(Value::String(format!("{number}{suffix}")))
}

pub(crate) fn singularize(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(value);
    let lower = source.to_lowercase();
    let result = if lower.ends_with("ies") && source.chars().count() > 3 {
        format!("{}y", char_slice(&source, 0, source.chars().count() - 3))
    } else if ["ses", "xes", "zes", "ches", "shes"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
    {
        char_slice(&source, 0, source.chars().count().saturating_sub(2))
    } else if lower.ends_with('s') && !lower.ends_with("ss") {
        char_slice(&source, 0, source.chars().count().saturating_sub(1))
    } else {
        source
    };
    Ok(Value::String(result))
}

pub(crate) fn pluralize(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(value);
    if source.is_empty() {
        return Ok(Value::String(source));
    }
    if singularize(&Value::String(source.clone()))? != Value::String(source.clone()) {
        return Ok(Value::String(source));
    }
    let lower = source.to_lowercase();
    let result = if ["s", "x", "z", "ch", "sh"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
    {
        format!("{source}es")
    } else {
        let chars: Vec<char> = lower.chars().collect();
        if chars.len() >= 2
            && chars.last() == Some(&'y')
            && !matches!(chars[chars.len() - 2], 'a' | 'e' | 'i' | 'o' | 'u')
        {
            format!(
                "{}ies",
                char_slice(&source, 0, source.chars().count().saturating_sub(1))
            )
        } else {
            format!("{source}s")
        }
    };
    Ok(Value::String(result))
}

pub(crate) fn reverse_string(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    Ok(Value::String(
        as_dataweave_string(value).chars().rev().collect(),
    ))
}

pub(crate) fn words(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    Ok(Value::Array(
        as_dataweave_string(value)
            .split_whitespace()
            .map(|word| Value::String(word.to_string()))
            .collect(),
    ))
}

pub(crate) fn repeat_string(text: &Value, times: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let count = numeric_value(times)? as i64;
    if count <= 0 {
        return Ok(Value::String(String::new()));
    }
    Ok(Value::String(
        as_dataweave_string(text).repeat(count as usize),
    ))
}

pub(crate) fn replace_all(
    text: &Value,
    target: &Value,
    replacement: &Value,
) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let needle = if target.is_null() {
        String::new()
    } else {
        as_dataweave_string(target)
    };
    if needle.is_empty() {
        return Ok(Value::String(source));
    }
    let replacement = if replacement.is_null() {
        String::new()
    } else {
        as_dataweave_string(replacement)
    };
    Ok(Value::String(source.replace(&needle, &replacement)))
}

pub(crate) fn replace_with(
    text: &Value,
    target: &Value,
    replacement: &Value,
) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let needle = if target.is_null() {
        String::new()
    } else {
        as_dataweave_string(target)
    };
    if needle.is_empty() {
        return Ok(Value::String(source));
    }
    let replacement = if replacement.is_null() {
        String::new()
    } else {
        as_dataweave_string(replacement)
    };
    let regex_pattern = regex_literal_inner(&needle).unwrap_or(&needle);
    if regex_literal_inner(&needle).is_some() || is_regex_like_pattern(regex_pattern) {
        let regex = Regex::new(regex_pattern).map_err(|err| {
            DwError::UnsupportedFeature(format!("regex pattern /{regex_pattern}/: {err}"))
        })?;
        return Ok(Value::String(
            regex
                .replace_all(&source, replacement.as_str())
                .into_owned(),
        ));
    }
    Ok(Value::String(source.replace(&needle, &replacement)))
}

fn is_regex_like_pattern(pattern: &str) -> bool {
    pattern.contains([
        '^', '$', '.', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '\\',
    ])
}

pub(crate) fn regex_literal_inner(pattern: &str) -> Option<&str> {
    pattern
        .strip_prefix('/')
        .and_then(|value| value.strip_suffix('/'))
}

pub(crate) fn remove_string(text: &Value, target: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let needle = if target.is_null() {
        String::new()
    } else {
        as_dataweave_string(target)
    };
    if needle.is_empty() {
        return Ok(Value::String(source));
    }
    Ok(Value::String(source.replace(&needle, "")))
}

pub(crate) fn substring_after(text: &Value, separator: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let marker = if separator.is_null() {
        String::new()
    } else {
        as_dataweave_string(separator)
    };
    if marker.is_empty() {
        return Ok(Value::String(source));
    }
    Ok(Value::String(
        source
            .find(&marker)
            .map(|index| source[index + marker.len()..].to_string())
            .unwrap_or_default(),
    ))
}

pub(crate) fn substring_after_last(text: &Value, separator: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let marker = if separator.is_null() {
        String::new()
    } else {
        as_dataweave_string(separator)
    };
    if marker.is_empty() {
        return Ok(Value::Null);
    }
    Ok(Value::String(
        source
            .rfind(&marker)
            .map(|index| source[index + marker.len()..].to_string())
            .unwrap_or_default(),
    ))
}

pub(crate) fn substring_before(text: &Value, separator: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let marker = if separator.is_null() {
        String::new()
    } else {
        as_dataweave_string(separator)
    };
    if marker.is_empty() {
        return Ok(Value::String(String::new()));
    }
    Ok(Value::String(
        source
            .find(&marker)
            .map(|index| source[..index].to_string())
            .unwrap_or_default(),
    ))
}

pub(crate) fn substring_before_last(text: &Value, separator: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let marker = if separator.is_null() {
        String::new()
    } else {
        as_dataweave_string(separator)
    };
    if marker.is_empty() {
        let len = source.chars().count();
        return Ok(Value::String(char_slice(&source, 0, len.saturating_sub(1))));
    }
    Ok(Value::String(
        source
            .rfind(&marker)
            .map(|index| source[..index].to_string())
            .unwrap_or_default(),
    ))
}

pub(crate) fn substring_every(text: &Value, amount: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let size = numeric_value(amount)?.floor() as i64;
    if size <= 0 {
        return Ok(Value::Array(Vec::new()));
    }
    let chars: Vec<char> = source.chars().collect();
    Ok(Value::Array(
        chars
            .chunks(size as usize)
            .map(|chunk| Value::String(chunk.iter().collect()))
            .collect(),
    ))
}

pub(crate) fn with_max_size(text: &Value, max_length: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let limit = numeric_value(max_length)?.floor() as i64;
    if limit <= 0 {
        return Ok(Value::String(source));
    }
    let len = source.chars().count();
    if len <= limit as usize {
        return Ok(Value::String(source));
    }
    Ok(Value::String(char_slice(&source, 0, limit as usize)))
}

pub(crate) fn unwrap_string(text: &Value, wrapper: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let token = if wrapper.is_null() {
        String::new()
    } else {
        as_dataweave_string(wrapper)
    };
    if token.is_empty() {
        return Ok(Value::String(source));
    }
    let Some(stripped_prefix) = source.strip_prefix(&token) else {
        return Ok(Value::String(source));
    };
    let Some(inner) = stripped_prefix.strip_suffix(&token) else {
        return Ok(Value::String(source));
    };
    Ok(Value::String(inner.to_string()))
}

pub(crate) fn wrap_with(text: &Value, wrapper: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let token = if wrapper.is_null() {
        String::new()
    } else {
        as_dataweave_string(wrapper)
    };
    Ok(Value::String(format!(
        "{token}{}{token}",
        as_dataweave_string(text)
    )))
}

pub(crate) fn wrap_if_missing(text: &Value, wrapper: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let token = if wrapper.is_null() {
        String::new()
    } else {
        as_dataweave_string(wrapper)
    };
    let mut source = as_dataweave_string(text);
    if token.is_empty() {
        return Ok(Value::String(source));
    }
    if source.is_empty() {
        return Ok(Value::String(token));
    }
    if !source.starts_with(&token) {
        source = format!("{token}{source}");
    }
    if !source.ends_with(&token) {
        source = format!("{source}{token}");
    }
    Ok(Value::String(source))
}
