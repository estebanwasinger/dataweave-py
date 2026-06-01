use serde_json::Map;
use serde_json::Value;

pub(crate) fn type_descriptor_from_value(value: &Value) -> Value {
    match value {
        Value::Null => type_null(),
        Value::Bool(_) => type_boolean(),
        Value::Number(_) => type_number(),
        Value::String(_) => type_string(),
        Value::Array(items) => {
            if items.is_empty() {
                type_array(type_any())
            } else {
                type_array(type_union(
                    items.iter().map(type_descriptor_from_value).collect(),
                ))
            }
        }
        Value::Object(map) => type_object(
            map.iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        field_descriptor(type_descriptor_from_value(value), false, false),
                    )
                })
                .collect(),
            true,
        ),
    }
}

pub(crate) fn type_kind(value: &Value) -> Option<&str> {
    value.get("kind").and_then(Value::as_str)
}

pub(crate) fn type_any() -> Value {
    Value::Object(Map::from_iter([(
        "kind".to_string(),
        Value::String("Any".to_string()),
    )]))
}

pub(crate) fn type_string() -> Value {
    Value::Object(Map::from_iter([(
        "kind".to_string(),
        Value::String("String".to_string()),
    )]))
}

pub(crate) fn type_number() -> Value {
    Value::Object(Map::from_iter([(
        "kind".to_string(),
        Value::String("Number".to_string()),
    )]))
}

pub(crate) fn type_boolean() -> Value {
    Value::Object(Map::from_iter([(
        "kind".to_string(),
        Value::String("Boolean".to_string()),
    )]))
}

pub(crate) fn type_null() -> Value {
    Value::Object(Map::from_iter([(
        "kind".to_string(),
        Value::String("Null".to_string()),
    )]))
}

pub(crate) fn type_array(element: Value) -> Value {
    Value::Object(Map::from_iter([
        ("kind".to_string(), Value::String("Array".to_string())),
        ("element".to_string(), element),
    ]))
}

pub(crate) fn type_object(fields: Map<String, Value>, open: bool) -> Value {
    Value::Object(Map::from_iter([
        ("kind".to_string(), Value::String("Object".to_string())),
        ("fields".to_string(), Value::Object(fields)),
        ("open".to_string(), Value::Bool(open)),
    ]))
}

pub(crate) fn field_descriptor(type_descriptor: Value, optional: bool, repeatable: bool) -> Value {
    Value::Object(Map::from_iter([
        ("type".to_string(), type_descriptor),
        ("optional".to_string(), Value::Bool(optional)),
        ("repeatable".to_string(), Value::Bool(repeatable)),
    ]))
}

pub(crate) fn type_union(types: Vec<Value>) -> Value {
    let mut unique = Vec::<Value>::new();
    for type_descriptor in types {
        if type_kind(&type_descriptor) == Some("Union") {
            if let Some(options) = type_descriptor.get("options").and_then(Value::as_array) {
                for option in options {
                    if !unique.iter().any(|existing| existing == option) {
                        unique.push(option.clone());
                    }
                }
            }
        } else if !unique.iter().any(|existing| existing == &type_descriptor) {
            unique.push(type_descriptor);
        }
    }
    if unique.is_empty() {
        return type_any();
    }
    if unique.iter().any(|value| type_kind(value) == Some("Any")) {
        return type_any();
    }
    if unique.len() == 1 {
        return unique.remove(0);
    }
    Value::Object(Map::from_iter([
        ("kind".to_string(), Value::String("Union".to_string())),
        ("options".to_string(), Value::Array(unique)),
    ]))
}
