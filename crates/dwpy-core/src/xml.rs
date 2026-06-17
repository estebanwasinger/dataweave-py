use serde_json::Map;
use serde_json::Value;
use std::collections::HashMap;

use crate::selectors::{unwrap_metadata_value, value_with_metadata};
use crate::{as_dataweave_string, output_option, DwError};

const XML_LIST_KEY: &str = "__dwpy_xml_list";

pub(crate) fn parse_xml_document(source: &str) -> Result<Value, DwError> {
    let mut parser = XmlParser::new(source);
    parser.skip_misc();
    let (name, value) = parser.parse_element()?;
    parser.skip_misc();
    if !parser.is_eof() {
        return Err(DwError::Parse("trailing XML content".to_string()));
    }
    let mut value = Value::Object(Map::from_iter([(name, value)]));
    expand_xml_namespaces(&mut value, &HashMap::new());
    collapse_xsi_nil_nodes(&mut value);
    if let Some(doc_type) = parser.doc_type {
        value = value_with_metadata(value, Map::from_iter([("docType".to_string(), doc_type)]));
    }
    Ok(value)
}

struct XmlParser<'a> {
    source: &'a str,
    index: usize,
    doc_type: Option<Value>,
}

impl<'a> XmlParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            index: 0,
            doc_type: None,
        }
    }

    fn is_eof(&self) -> bool {
        self.index >= self.source.len()
    }

    fn rest(&self) -> &'a str {
        &self.source[self.index..]
    }

    fn skip_misc(&mut self) {
        loop {
            self.skip_whitespace();
            if self.rest().starts_with("<?") {
                if let Some(end) = self.rest().find("?>") {
                    self.index += end + 2;
                    continue;
                }
            }
            if self.rest().starts_with("<!--") {
                if let Some(end) = self.rest().find("-->") {
                    self.index += end + 3;
                    continue;
                }
            }
            if self.rest().starts_with("<!DOCTYPE") {
                if let Some(end) = self.rest().find('>') {
                    let declaration = &self.rest()[..=end];
                    self.doc_type = parse_doc_type(declaration);
                    self.index += end + 1;
                    continue;
                }
            }
            break;
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.rest().chars().next() {
            if !ch.is_whitespace() {
                break;
            }
            self.index += ch.len_utf8();
        }
    }

    fn consume(&mut self, token: &str) -> bool {
        if self.rest().starts_with(token) {
            self.index += token.len();
            true
        } else {
            false
        }
    }

    fn parse_element(&mut self) -> Result<(String, Value), DwError> {
        if !self.consume("<") {
            return Err(DwError::Parse("expected XML element".to_string()));
        }
        if self.rest().starts_with('/') {
            return Err(DwError::Parse("unexpected XML closing tag".to_string()));
        }

        let name = self.parse_name()?;
        let mut attributes = Vec::new();
        loop {
            self.skip_whitespace();
            if self.consume("/>") {
                return Ok((name, xml_node_value(attributes, Vec::new(), String::new())));
            }
            if self.consume(">") {
                break;
            }
            let attr_name = self.parse_name()?;
            self.skip_whitespace();
            if !self.consume("=") {
                return Err(DwError::Parse(format!(
                    "expected value for XML attribute {attr_name}"
                )));
            }
            self.skip_whitespace();
            let attr_value = self.parse_quoted_value()?;
            attributes.push((format!("@{attr_name}"), Value::String(attr_value)));
        }

        let mut children = Vec::new();
        let mut text = String::new();
        loop {
            if self.rest().starts_with("</") {
                self.index += 2;
                let closing_name = self.parse_name()?;
                self.skip_whitespace();
                if !self.consume(">") {
                    return Err(DwError::Parse(format!(
                        "expected end of XML closing tag {closing_name}"
                    )));
                }
                if closing_name != name {
                    return Err(DwError::Parse(format!(
                        "mismatched XML tag {name} closed by {closing_name}"
                    )));
                }
                return Ok((name, xml_node_value(attributes, children, text)));
            }
            if self.rest().starts_with("<!--") {
                if let Some(end) = self.rest().find("-->") {
                    self.index += end + 3;
                    continue;
                }
                return Err(DwError::Parse("unterminated XML comment".to_string()));
            }
            if self.rest().starts_with("<![CDATA[") {
                let Some(end) = self.rest().find("]]>") else {
                    return Err(DwError::Parse("unterminated XML CDATA".to_string()));
                };
                text.push_str(&self.rest()["<![CDATA[".len()..end]);
                self.index += end + "]]>".len();
                continue;
            }
            if self.rest().starts_with('<') {
                children.push(self.parse_element()?);
                continue;
            }
            if self.is_eof() {
                return Err(DwError::Parse(format!("unclosed XML tag {name}")));
            }
            let next_tag = self
                .rest()
                .find('<')
                .ok_or_else(|| DwError::Parse(format!("unclosed XML tag {name}")))?;
            text.push_str(&decode_xml_entities(&self.rest()[..next_tag]));
            self.index += next_tag;
        }
    }

    fn parse_name(&mut self) -> Result<String, DwError> {
        let start = self.index;
        while let Some(ch) = self.rest().chars().next() {
            if ch.is_whitespace() || matches!(ch, '/' | '>' | '=' | '?' | '<') {
                break;
            }
            self.index += ch.len_utf8();
        }
        if self.index == start {
            return Err(DwError::Parse("expected XML name".to_string()));
        }
        Ok(self.source[start..self.index].to_string())
    }

    fn parse_quoted_value(&mut self) -> Result<String, DwError> {
        let Some(quote) = self.rest().chars().next() else {
            return Err(DwError::Parse("expected XML attribute quote".to_string()));
        };
        if quote != '"' && quote != '\'' {
            return Err(DwError::Parse("expected XML attribute quote".to_string()));
        }
        self.index += quote.len_utf8();
        let start = self.index;
        while let Some(ch) = self.rest().chars().next() {
            if ch == quote {
                let value = decode_xml_entities(&self.source[start..self.index]);
                self.index += quote.len_utf8();
                return Ok(value);
            }
            self.index += ch.len_utf8();
        }
        Err(DwError::Parse("unterminated XML attribute".to_string()))
    }
}

fn parse_doc_type(source: &str) -> Option<Value> {
    let inner = source
        .trim()
        .strip_prefix("<!DOCTYPE")?
        .trim()
        .strip_suffix('>')?
        .trim();
    let mut parts = inner.split_whitespace();
    let root_name = parts.next()?.to_string();
    let kind = parts.next()?;
    let mut map = Map::from_iter([("rootName".to_string(), Value::String(root_name))]);
    match kind {
        "SYSTEM" => {
            let system_id = parse_doctype_quoted(inner)?;
            map.insert("systemId".to_string(), Value::String(system_id));
        }
        "PUBLIC" => {
            let quoted = parse_all_doctype_quoted(inner);
            if let Some(public_id) = quoted.first() {
                map.insert("publicId".to_string(), Value::String(public_id.clone()));
            }
            if let Some(system_id) = quoted.get(1) {
                map.insert("systemId".to_string(), Value::String(system_id.clone()));
            }
        }
        _ => return None,
    }
    Some(Value::Object(map))
}

fn parse_doctype_quoted(source: &str) -> Option<String> {
    parse_all_doctype_quoted(source).into_iter().next()
}

fn parse_all_doctype_quoted(source: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut index = 0usize;
    while index < source.len() {
        let ch = source[index..].chars().next().unwrap_or_default();
        if ch != '"' && ch != '\'' {
            index += ch.len_utf8();
            continue;
        }
        let quote = ch;
        index += quote.len_utf8();
        let start = index;
        while index < source.len() {
            let ch = source[index..].chars().next().unwrap_or_default();
            if ch == quote {
                values.push(source[start..index].to_string());
                index += quote.len_utf8();
                break;
            }
            index += ch.len_utf8();
        }
    }
    values
}

fn xml_node_value(
    attributes: Vec<(String, Value)>,
    children: Vec<(String, Value)>,
    text: String,
) -> Value {
    let trimmed_text = text.trim();
    if attributes.is_empty() && children.is_empty() {
        return Value::String(trimmed_text.to_string());
    }

    let mut map = Map::new();
    for (key, value) in attributes {
        map.insert(key, value);
    }
    for (key, value) in children {
        if let Some(existing) = map.get_mut(&key) {
            if let Some(items) = xml_list_items_mut(existing) {
                items.push(value);
            } else {
                let first = existing.clone();
                *existing = xml_list_value(vec![first, value]);
            }
        } else {
            map.insert(key, value);
        }
    }
    if !trimmed_text.is_empty() {
        map.insert("#text".to_string(), Value::String(trimmed_text.to_string()));
    }
    Value::Object(map)
}

fn xml_list_value(items: Vec<Value>) -> Value {
    Value::Object(Map::from_iter([(
        XML_LIST_KEY.to_string(),
        Value::Array(items),
    )]))
}

pub(crate) fn xml_list_items(value: &Value) -> Option<&Vec<Value>> {
    let Value::Object(map) = value else {
        return None;
    };
    if map.len() != 1 {
        return None;
    }
    let Some(Value::Array(items)) = map.get(XML_LIST_KEY) else {
        return None;
    };
    Some(items)
}

fn xml_list_items_mut(value: &mut Value) -> Option<&mut Vec<Value>> {
    let Value::Object(map) = value else {
        return None;
    };
    if map.len() != 1 {
        return None;
    }
    let Some(Value::Array(items)) = map.get_mut(XML_LIST_KEY) else {
        return None;
    };
    Some(items)
}

fn expand_xml_namespaces(value: &mut Value, inherited: &HashMap<String, String>) {
    if let Some(items) = xml_list_items_mut(value) {
        for item in items {
            expand_xml_namespaces(item, inherited);
        }
        return;
    }

    let Value::Object(map) = value else {
        return;
    };

    let mut namespaces = inherited.clone();
    for (key, val) in map.iter() {
        if key == "@xmlns" {
            namespaces.insert(String::new(), as_dataweave_string(val));
        } else if let Some(prefix) = key.strip_prefix("@xmlns:") {
            namespaces.insert(prefix.to_string(), as_dataweave_string(val));
        }
    }

    let mut rebuilt = Map::new();
    for (key, mut child) in std::mem::take(map) {
        if key == "@xmlns" || key.starts_with("@xmlns:") {
            continue;
        }
        let child_namespaces = if let Value::Object(child_map) = &child {
            let mut child_namespaces = namespaces.clone();
            for (child_key, child_value) in child_map {
                if child_key == "@xmlns" {
                    child_namespaces.insert(String::new(), as_dataweave_string(child_value));
                } else if let Some(prefix) = child_key.strip_prefix("@xmlns:") {
                    child_namespaces.insert(prefix.to_string(), as_dataweave_string(child_value));
                }
            }
            child_namespaces
        } else {
            namespaces.clone()
        };
        let expanded_key = expand_xml_name(&key, &child_namespaces);
        expand_xml_namespaces(&mut child, &child_namespaces);
        insert_xml_child(&mut rebuilt, expanded_key, child);
    }
    *map = rebuilt;
}

fn expand_xml_name(name: &str, namespaces: &HashMap<String, String>) -> String {
    let (attribute_prefix, candidate) = name
        .strip_prefix('@')
        .map_or(("", name), |name| ("@", name));
    let Some((prefix, local)) = candidate.split_once(':') else {
        if attribute_prefix.is_empty() {
            if let Some(uri) = namespaces.get("") {
                return format!("{{{uri}}}{candidate}");
            }
        }
        return name.to_string();
    };
    let Some(uri) = namespaces.get(prefix) else {
        return name.to_string();
    };
    format!("{attribute_prefix}{{{uri}}}{local}")
}

fn insert_xml_child(map: &mut Map<String, Value>, key: String, value: Value) {
    if let Some(existing) = map.get_mut(&key) {
        if let Some(items) = xml_list_items_mut(existing) {
            items.push(value);
        } else {
            let first = existing.clone();
            *existing = xml_list_value(vec![first, value]);
        }
    } else {
        map.insert(key, value);
    }
}

fn collapse_xsi_nil_nodes(value: &mut Value) {
    if let Some(items) = xml_list_items_mut(value) {
        for item in items {
            collapse_xsi_nil_nodes(item);
        }
        return;
    }

    let Value::Object(map) = value else {
        return;
    };

    for child in map.values_mut() {
        collapse_xsi_nil_nodes(child);
    }

    let xsi_nil = map
        .iter()
        .any(|(key, value)| xml_local_name(key) == "nil" && as_dataweave_string(value) == "true");
    let has_content = map
        .iter()
        .any(|(key, _)| !key.starts_with('@') && key != "#text");
    let has_text = map
        .get("#text")
        .is_some_and(|text| !as_dataweave_string(text).trim().is_empty());
    if xsi_nil && !has_content && !has_text {
        *value = Value::Null;
    }
}

pub(crate) fn render_xml_output(value: &Value, directive: &str) -> Result<String, DwError> {
    let (root_name, root_value) = match value {
        Value::Object(map) if map.len() == 1 && output_option(directive, "root").is_none() => {
            map.iter().next().expect("checked one item")
        }
        _ => {
            let root = output_option(directive, "root").unwrap_or("root");
            return Ok(render_xml_element(root, value));
        }
    };
    Ok(render_xml_element(root_name, root_value))
}

fn render_xml_element(name: &str, value: &Value) -> String {
    if let Some(unwrapped) = unwrap_metadata_value(value) {
        return match unwrapped {
            Value::Object(_) | Value::Array(_) => render_xml_element(name, &unwrapped),
            _ => format!(
                "<{name}>{}</{name}>",
                escape_xml_text(&as_dataweave_string(value))
            ),
        };
    }
    if let Some(items) = xml_list_items(value) {
        return items
            .iter()
            .map(|item| render_xml_element(name, item))
            .collect::<String>();
    }
    match value {
        Value::Object(map) => {
            let mut attributes = String::new();
            let mut text = String::new();
            let mut children = String::new();
            for (key, child_value) in map {
                if let Some(attribute) = key.strip_prefix('@') {
                    attributes.push(' ');
                    attributes.push_str(attribute);
                    attributes.push_str("=\"");
                    attributes.push_str(&escape_xml_attr(&as_dataweave_string(child_value)));
                    attributes.push('"');
                } else if key == "#text" {
                    text.push_str(&escape_xml_text(&as_dataweave_string(child_value)));
                } else {
                    children.push_str(&render_xml_element(key, child_value));
                }
            }
            if text.is_empty() && children.is_empty() {
                format!("<{name}{attributes} />")
            } else {
                format!("<{name}{attributes}>{text}{children}</{name}>")
            }
        }
        Value::Array(items) => {
            let children = items
                .iter()
                .map(|item| render_xml_element("item", item))
                .collect::<String>();
            format!("<{name}>{children}</{name}>")
        }
        Value::Null => format!("<{name} />"),
        other => format!(
            "<{name}>{}</{name}>",
            escape_xml_text(&as_dataweave_string(other))
        ),
    }
}

fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_attr(value: &str) -> String {
    escape_xml_text(value).replace('"', "&quot;")
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

pub(crate) fn matching_xml_key<'a>(map: &'a Map<String, Value>, part: &str) -> Option<&'a String> {
    map.keys().find(|key| xml_local_name(key) == part)
}

pub(crate) fn xml_namespace_uri(key: &str) -> Option<&str> {
    key.strip_prefix('{')
        .and_then(|value| value.split_once('}'))
        .map(|(namespace, _)| namespace)
}

pub(crate) fn xml_local_name(key: &str) -> &str {
    let key = key.strip_prefix('@').unwrap_or(key);
    if let Some((_, local)) = key
        .strip_prefix('{')
        .and_then(|value| value.split_once('}'))
    {
        return local;
    }
    key.rsplit_once(':')
        .map(|(_prefix, local)| local)
        .unwrap_or(key)
}
