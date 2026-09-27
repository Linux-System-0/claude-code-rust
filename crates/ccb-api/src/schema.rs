//! Runtime JSON-Schema validation for tool inputs (Phase 2 / T2.3).
//!
//! The TS implementation validated every tool input with a `zod` schema. On the
//! Rust side built-in tools declare a JSON Schema (`ToolSpec::input_schema`) and
//! this module checks model-provided arguments against it before dispatch.
//!
//! It implements the subset of JSON Schema that tool definitions actually use:
//! `type`, `enum`, `const`, `$ref` (local), the `allOf`/`anyOf`/`oneOf`/`not`
//! combinators, string/number/array/object constraints, and
//! `properties`/`required`/`additionalProperties`/`patternProperties`.
//! Unknown keywords are ignored, as the specification requires, so richer
//! third-party (MCP) schemas still validate the parts we understand instead of
//! rejecting the whole input.

use regex::Regex;
use serde_json::{Map, Value};
use thiserror::Error;

/// A validation failure, located at a JSON path relative to the root instance.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{path}: {message} (keyword `{keyword}`)")]
pub struct ValidationError {
    /// JSON path such as `$`, `$.file_path`, or `$.items[0]`.
    pub path: String,
    /// Human-readable description of the failure.
    pub message: String,
    /// The JSON Schema keyword that rejected the instance.
    pub keyword: &'static str,
}

impl ValidationError {
    fn new(path: &str, message: impl Into<String>, keyword: &'static str) -> Self {
        ValidationError {
            path: path.to_owned(),
            message: message.into(),
            keyword,
        }
    }
}

/// Validate `instance` against a JSON Schema document.
///
/// Returns `Ok(())` when the instance is valid. The schema may be a boolean
/// schema (`true` accepts everything, `false` rejects everything).
pub fn validate(schema: &Value, instance: &Value) -> Result<(), ValidationError> {
    validate_at(schema, instance, schema, "$")
}

/// Shorthand for validating a tool input against a schema.
pub fn validate_tool_input(schema: &Value, input: &Value) -> Result<(), ValidationError> {
    validate(schema, input)
}

fn validate_at(
    schema: &Value,
    instance: &Value,
    root: &Value,
    path: &str,
) -> Result<(), ValidationError> {
    match schema {
        Value::Bool(true) => return Ok(()),
        Value::Bool(false) => {
            return Err(ValidationError::new(path, "value is not allowed", "false"));
        }
        Value::Object(_) => {}
        other => {
            return Err(ValidationError::new(
                path,
                format!("schema must be an object or boolean, found {other}"),
                "$schema",
            ));
        }
    }
    let object = schema.as_object().expect("checked above");

    if let Some(reference) = object.get("$ref") {
        let reference = reference
            .as_str()
            .ok_or_else(|| ValidationError::new(path, "$ref must be a string", "$ref"))?;
        let target = resolve_local_ref(root, reference).ok_or_else(|| {
            ValidationError::new(path, format!("unresolvable $ref `{reference}`"), "$ref")
        })?;
        return validate_at(target, instance, root, path);
    }

    if let Some(all_of) = object.get("allOf") {
        for (index, sub) in subschemas(all_of, path, "allOf")?.into_iter().enumerate() {
            validate_at(sub, instance, root, path).map_err(|err| {
                ValidationError::new(
                    path,
                    format!("allOf[{index}] failed: {}", err.message),
                    "allOf",
                )
            })?;
        }
    }

    if let Some(any_of) = object.get("anyOf") {
        let subs = subschemas(any_of, path, "anyOf")?;
        if !subs
            .iter()
            .any(|sub| validate_at(sub, instance, root, path).is_ok())
        {
            return Err(ValidationError::new(
                path,
                "value does not match any of the allowed schemas",
                "anyOf",
            ));
        }
    }

    if let Some(one_of) = object.get("oneOf") {
        let subs = subschemas(one_of, path, "oneOf")?;
        let matches = subs
            .iter()
            .filter(|sub| validate_at(sub, instance, root, path).is_ok())
            .count();
        if matches != 1 {
            return Err(ValidationError::new(
                path,
                format!("value matches {matches} schemas but exactly one is required"),
                "oneOf",
            ));
        }
    }

    if let Some(not) = object.get("not") {
        if validate_at(not, instance, root, path).is_ok() {
            return Err(ValidationError::new(
                path,
                "value matches a forbidden schema",
                "not",
            ));
        }
    }

    if let Some(enum_values) = object.get("enum") {
        let values = enum_values
            .as_array()
            .ok_or_else(|| ValidationError::new(path, "enum must be an array", "enum"))?;
        if !values.iter().any(|candidate| candidate == instance) {
            return Err(ValidationError::new(
                path,
                "value is not one of the allowed enum values",
                "enum",
            ));
        }
    }

    if let Some(constant) = object.get("const") {
        if constant != instance {
            return Err(ValidationError::new(
                path,
                format!("value must equal the constant {constant}"),
                "const",
            ));
        }
    }

    if let Some(type_schema) = object.get("type") {
        if !type_matches(type_schema, instance, path)? {
            return Err(ValidationError::new(
                path,
                format!(
                    "expected type {}, found {}",
                    describe_type(type_schema),
                    instance_type(instance)
                ),
                "type",
            ));
        }
    }

    match instance {
        Value::String(text) => validate_string(object, text, path)?,
        Value::Number(number) => validate_number(object, number, path)?,
        Value::Array(items) => validate_array(object, items, root, path)?,
        Value::Object(map) => validate_object(object, map, root, path)?,
        Value::Null | Value::Bool(_) => {}
    }

    Ok(())
}

fn validate_string(
    schema: &Map<String, Value>,
    text: &str,
    path: &str,
) -> Result<(), ValidationError> {
    let length = text.chars().count() as u64;
    if let Some(min) = schema.get("minLength").and_then(Value::as_u64) {
        if length < min {
            return Err(ValidationError::new(
                path,
                format!("string is shorter than minLength {min}"),
                "minLength",
            ));
        }
    }
    if let Some(max) = schema.get("maxLength").and_then(Value::as_u64) {
        if length > max {
            return Err(ValidationError::new(
                path,
                format!("string is longer than maxLength {max}"),
                "maxLength",
            ));
        }
    }
    if let Some(pattern) = schema.get("pattern") {
        let pattern = pattern
            .as_str()
            .ok_or_else(|| ValidationError::new(path, "pattern must be a string", "pattern"))?;
        let regex = compile_regex(pattern, path)?;
        if !regex.is_match(text) {
            return Err(ValidationError::new(
                path,
                format!("string does not match pattern `{pattern}`"),
                "pattern",
            ));
        }
    }
    Ok(())
}

fn validate_number(
    schema: &Map<String, Value>,
    number: &serde_json::Number,
    path: &str,
) -> Result<(), ValidationError> {
    let value = number.as_f64().unwrap_or(f64::NAN);
    check_bound(schema, "minimum", value, path, |v, bound| v < bound)?;
    check_bound(schema, "maximum", value, path, |v, bound| v > bound)?;
    check_bound(schema, "exclusiveMinimum", value, path, |v, bound| {
        v <= bound
    })?;
    check_bound(schema, "exclusiveMaximum", value, path, |v, bound| {
        v >= bound
    })?;
    if let Some(multiple) = schema.get("multipleOf").and_then(Value::as_f64) {
        if multiple == 0.0 {
            return Err(ValidationError::new(
                path,
                "multipleOf must not be zero",
                "multipleOf",
            ));
        }
        let ratio = value / multiple;
        if (ratio - ratio.round()).abs() > 1e-9 {
            return Err(ValidationError::new(
                path,
                format!("number is not a multiple of {multiple}"),
                "multipleOf",
            ));
        }
    }
    Ok(())
}

fn check_bound(
    schema: &Map<String, Value>,
    keyword: &'static str,
    value: f64,
    path: &str,
    violates: impl Fn(f64, f64) -> bool,
) -> Result<(), ValidationError> {
    if let Some(bound) = schema.get(keyword).and_then(Value::as_f64) {
        if violates(value, bound) {
            return Err(ValidationError::new(
                path,
                format!("number violates {keyword} {bound}"),
                keyword,
            ));
        }
    }
    Ok(())
}

fn validate_array(
    schema: &Map<String, Value>,
    items: &[Value],
    root: &Value,
    path: &str,
) -> Result<(), ValidationError> {
    if let Some(min) = schema.get("minItems").and_then(Value::as_u64) {
        if (items.len() as u64) < min {
            return Err(ValidationError::new(
                path,
                format!("array has fewer than minItems {min}"),
                "minItems",
            ));
        }
    }
    if let Some(max) = schema.get("maxItems").and_then(Value::as_u64) {
        if (items.len() as u64) > max {
            return Err(ValidationError::new(
                path,
                format!("array has more than maxItems {max}"),
                "maxItems",
            ));
        }
    }
    if schema.get("uniqueItems").and_then(Value::as_bool) == Some(true) {
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                if items[i] == items[j] {
                    return Err(ValidationError::new(
                        path,
                        format!("array items at [{i}] and [{j}] are not unique"),
                        "uniqueItems",
                    ));
                }
            }
        }
    }
    match schema.get("items") {
        Some(Value::Array(tuple)) => {
            for (index, sub) in tuple.iter().enumerate() {
                if let Some(item) = items.get(index) {
                    validate_at(sub, item, root, &join_index(path, index))?;
                }
            }
        }
        Some(sub) => {
            for (index, item) in items.iter().enumerate() {
                validate_at(sub, item, root, &join_index(path, index))?;
            }
        }
        None => {}
    }
    Ok(())
}

fn validate_object(
    schema: &Map<String, Value>,
    map: &Map<String, Value>,
    root: &Value,
    path: &str,
) -> Result<(), ValidationError> {
    if let Some(min) = schema.get("minProperties").and_then(Value::as_u64) {
        if (map.len() as u64) < min {
            return Err(ValidationError::new(
                path,
                format!("object has fewer than minProperties {min}"),
                "minProperties",
            ));
        }
    }
    if let Some(max) = schema.get("maxProperties").and_then(Value::as_u64) {
        if (map.len() as u64) > max {
            return Err(ValidationError::new(
                path,
                format!("object has more than maxProperties {max}"),
                "maxProperties",
            ));
        }
    }

    let properties = schema.get("properties").and_then(Value::as_object);
    let patterns = compiled_pattern_properties(schema, path)?;

    if let Some(required) = schema.get("required") {
        let required = required
            .as_array()
            .ok_or_else(|| ValidationError::new(path, "required must be an array", "required"))?;
        for name in required.iter().filter_map(Value::as_str) {
            if !map.contains_key(name) {
                return Err(ValidationError::new(
                    &join_key(path, name),
                    "required property is missing",
                    "required",
                ));
            }
        }
    }

    if let Some(properties) = properties {
        for (name, sub) in properties {
            if let Some(value) = map.get(name) {
                validate_at(sub, value, root, &join_key(path, name))?;
            }
        }
    }

    // Validate properties matched by `patternProperties`. Every matching
    // subschema applies, so iterate all of them.
    for (name, value) in map {
        for (regex, sub) in &patterns {
            if regex.is_match(name) {
                validate_at(sub, value, root, &join_key(path, name))?;
            }
        }
    }

    if let Some(additional) = schema.get("additionalProperties") {
        let known = |name: &str| {
            properties.is_some_and(|props| props.contains_key(name))
                || patterns.iter().any(|(regex, _)| regex.is_match(name))
        };
        for (name, value) in map {
            if known(name) {
                continue;
            }
            match additional {
                Value::Bool(false) => {
                    return Err(ValidationError::new(
                        &join_key(path, name),
                        "additional properties are not allowed",
                        "additionalProperties",
                    ));
                }
                Value::Bool(true) => {}
                sub => validate_at(sub, value, root, &join_key(path, name))?,
            }
        }
    }

    Ok(())
}

fn compiled_pattern_properties(
    schema: &Map<String, Value>,
    path: &str,
) -> Result<Vec<(Regex, Value)>, ValidationError> {
    let mut compiled = Vec::new();
    if let Some(patterns) = schema.get("patternProperties").and_then(Value::as_object) {
        for (pattern, sub) in patterns {
            compiled.push((compile_regex(pattern, path)?, sub.clone()));
        }
    }
    Ok(compiled)
}

fn compile_regex(pattern: &str, path: &str) -> Result<Regex, ValidationError> {
    Regex::new(pattern).map_err(|err| {
        ValidationError::new(
            path,
            format!("invalid regex pattern `{pattern}`: {err}"),
            "pattern",
        )
    })
}

fn subschemas<'a>(
    value: &'a Value,
    path: &str,
    keyword: &'static str,
) -> Result<Vec<&'a Value>, ValidationError> {
    value
        .as_array()
        .map(|items| items.iter().collect())
        .ok_or_else(|| ValidationError::new(path, format!("{keyword} must be an array"), keyword))
}

fn type_matches(
    type_schema: &Value,
    instance: &Value,
    path: &str,
) -> Result<bool, ValidationError> {
    match type_schema {
        Value::String(name) => Ok(matches_type(name, instance)),
        Value::Array(names) => {
            for name in names {
                let name = name.as_str().ok_or_else(|| {
                    ValidationError::new(path, "type entries must be strings", "type")
                })?;
                if matches_type(name, instance) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Err(ValidationError::new(
            path,
            "type must be a string or array of strings",
            "type",
        )),
    }
}

fn matches_type(name: &str, instance: &Value) -> bool {
    match name {
        "null" => instance.is_null(),
        "boolean" => instance.is_boolean(),
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "number" => instance.is_number(),
        "integer" => instance.as_f64().is_some_and(|value| value.fract() == 0.0),
        "string" => instance.is_string(),
        // Unknown type names are treated as non-matching rather than an error,
        // matching the lenient behaviour of `ajv` with unknown types.
        _ => false,
    }
}

fn instance_type(instance: &Value) -> &'static str {
    match instance {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn describe_type(type_schema: &Value) -> String {
    match type_schema {
        Value::Array(names) => names
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" | "),
        other => other.to_string(),
    }
}

fn resolve_local_ref<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let pointer = reference.strip_prefix("#/")?;
    let decoded = pointer.replace("~1", "/").replace("~0", "~");
    root.pointer(&format!("/{decoded}"))
}

fn join_key(path: &str, key: &str) -> String {
    format!("{path}.{key}")
}

fn join_index(path: &str, index: usize) -> String {
    format!("{path}[{index}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_valid_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "minLength": 1},
                "offset": {"type": "integer", "minimum": 0}
            },
            "required": ["file_path"],
            "additionalProperties": false
        });
        let input = json!({"file_path": "/tmp/a", "offset": 3});
        assert!(validate(&schema, &input).is_ok());
    }

    #[test]
    fn reports_missing_required_with_path() {
        let schema = json!({
            "type": "object",
            "properties": {"file_path": {"type": "string"}},
            "required": ["file_path"]
        });
        let err = validate(&schema, &json!({})).unwrap_err();
        assert_eq!(err.keyword, "required");
        assert_eq!(err.path, "$.file_path");
    }

    #[test]
    fn rejects_wrong_type_and_extra_property() {
        let schema = json!({
            "type": "object",
            "properties": {"count": {"type": "integer"}},
            "additionalProperties": false
        });
        let type_err = validate(&schema, &json!({"count": "many"})).unwrap_err();
        assert_eq!(type_err.keyword, "type");
        assert_eq!(type_err.path, "$.count");

        let extra_err = validate(&schema, &json!({"count": 1, "nope": true})).unwrap_err();
        assert_eq!(extra_err.keyword, "additionalProperties");
        assert_eq!(extra_err.path, "$.nope");
    }

    #[test]
    fn validates_enum_and_pattern() {
        let schema =
            json!({"type": "string", "enum": ["read", "write"], "pattern": "^(read|write)$"});
        assert!(validate(&schema, &json!("read")).is_ok());
        assert_eq!(
            validate(&schema, &json!("exec")).unwrap_err().keyword,
            "enum"
        );
    }

    #[test]
    fn validates_arrays_and_items() {
        let schema = json!({
            "type": "array",
            "items": {"type": "number"},
            "minItems": 1,
            "uniqueItems": true
        });
        assert!(validate(&schema, &json!([1, 2, 3])).is_ok());
        assert_eq!(
            validate(&schema, &json!([])).unwrap_err().keyword,
            "minItems"
        );
        assert_eq!(
            validate(&schema, &json!([1, 1])).unwrap_err().keyword,
            "uniqueItems"
        );
        assert_eq!(
            validate(&schema, &json!([1, "two"])).unwrap_err().path,
            "$[1]"
        );
    }

    #[test]
    fn validates_combinators() {
        let any_of = json!({"anyOf": [{"type": "string"}, {"type": "null"}]});
        assert!(validate(&any_of, &json!(null)).is_ok());
        assert!(validate(&any_of, &json!("ok")).is_ok());
        assert_eq!(validate(&any_of, &json!(1)).unwrap_err().keyword, "anyOf");

        let one_of = json!({"oneOf": [{"type": "number"}, {"type": "integer"}]});
        // 1 matches both `number` and `integer`, so `oneOf` fails.
        assert_eq!(validate(&one_of, &json!(1)).unwrap_err().keyword, "oneOf");
    }

    #[test]
    fn resolves_local_refs() {
        let schema = json!({
            "type": "object",
            "properties": {"mode": {"$ref": "#/$defs/mode"}},
            "$defs": {"mode": {"type": "string", "enum": ["fast", "slow"]}}
        });
        assert!(validate(&schema, &json!({"mode": "fast"})).is_ok());
        assert_eq!(
            validate(&schema, &json!({"mode": "other"}))
                .unwrap_err()
                .keyword,
            "enum"
        );
    }

    #[test]
    fn boolean_schema_and_unknown_keywords() {
        assert!(validate(&json!(true), &json!({"anything": 1})).is_ok());
        assert_eq!(
            validate(&json!(false), &json!(1)).unwrap_err().keyword,
            "false"
        );
        // Unknown keyword is ignored.
        let schema = json!({"type": "string", "x-custom": true});
        assert!(validate(&schema, &json!("hi")).is_ok());
    }

    #[test]
    fn pattern_properties_are_known_to_additional_properties() {
        let schema = json!({
            "type": "object",
            "patternProperties": {"^x-": {"type": "number"}},
            "additionalProperties": false
        });
        assert!(validate(&schema, &json!({"x-a": 1})).is_ok());
        assert_eq!(
            validate(&schema, &json!({"x-a": "no"}))
                .unwrap_err()
                .keyword,
            "type"
        );
        assert_eq!(
            validate(&schema, &json!({"y": 1})).unwrap_err().keyword,
            "additionalProperties"
        );
    }
}
