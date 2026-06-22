use serde_json::{Map, Value};

use crate::providers::ExtraBodyValueType;

/// Set a value at a dot-separated JSON path inside `obj`, creating intermediate
/// objects as needed. E.g. `set_nested_path(obj, "provider.only", val)` produces
/// `{"provider": {"only": val}}`.
pub fn set_nested_path(obj: &mut Map<String, Value>, path: &str, value: Value) {
    let mut parts = path.splitn(2, '.');
    let key = parts.next().unwrap_or(path);
    if let Some(rest) = parts.next() {
        let child = obj.entry(key.to_string()).or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(child_map) = child {
            set_nested_path(child_map, rest, value);
        } else {
            // Overwrite non-object with a new object containing our value
            let mut new_map = Map::new();
            set_nested_path(&mut new_map, rest, value);
            *child = Value::Object(new_map);
        }
    } else {
        obj.insert(key.to_string(), value);
    }
}

/// Navigate a dot-separated JSON path and return a reference to the leaf value.
pub fn get_nested_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut parts = path.splitn(2, '.');
    let key = parts.next()?;
    let child = value.get(key)?;
    if let Some(rest) = parts.next() {
        get_nested_path(child, rest)
    } else {
        Some(child)
    }
}

/// Convert user input text to a JSON `Value` based on the value type.
/// Returns `None` if the input is empty or (for Bool) unrecognized.
pub fn serialize_value(value_type: &ExtraBodyValueType, input: &str) -> Option<Value> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    match value_type {
        ExtraBodyValueType::String => Some(Value::String(trimmed.to_string())),
        ExtraBodyValueType::StringList => {
            let entries: Vec<Value> = trimmed
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| Value::String(s.to_string()))
                .collect();
            if entries.is_empty() { None } else { Some(Value::Array(entries)) }
        }
        ExtraBodyValueType::Bool => match trimmed.to_lowercase().as_str() {
            "true" | "yes" | "1" => Some(Value::Bool(true)),
            "false" | "no" | "0" => Some(Value::Bool(false)),
            _ => None,
        },
    }
}

/// Convert a JSON `Value` back to a display string based on the value type.
pub fn deserialize_value(value_type: &ExtraBodyValueType, value: &Value) -> Option<String> {
    match value_type {
        ExtraBodyValueType::String => value.as_str().map(|s| s.to_string()),
        ExtraBodyValueType::StringList => {
            let arr = value.as_array()?;
            let items: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
            Some(items.join(", "))
        }
        ExtraBodyValueType::Bool => value.as_bool().map(|b| if b { "true" } else { "false" }.to_string()),
    }
}

/// Build the `CLAUDE_CODE_EXTRA_BODY` JSON string from a `serde_json::Map`.
/// Returns `None` if the map is empty (so the env var is omitted).
pub fn build_extra_body(map: Map<String, Value>) -> Option<String> {
    if map.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&Value::Object(map)).unwrap())
    }
}

/// Parse an existing `CLAUDE_CODE_EXTRA_BODY` string into a `serde_json::Map`.
/// Returns an empty map if the string is absent, empty, or not valid JSON.
pub fn parse_extra_body(json: Option<&str>) -> Map<String, Value> {
    json.and_then(|s| serde_json::from_str(s).ok())
        .and_then(|v: Value| if let Value::Object(m) = v { Some(m) } else { None })
        .unwrap_or_default()
}
