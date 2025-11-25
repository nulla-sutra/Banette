use std::collections::HashMap;
use tera::{Result, Value, to_value};

/// Tera filter to check if a property is required.
///
/// Usage in the template: {{ prop_name | is_required(required_list=schema.required) }}
pub(crate) fn is_required_filter(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    // 1. Get the property name to check (prop_name)
    let prop_name = value.as_str().ok_or_else(|| {
        tera::Error::msg("is_required filter expects property name as input string.")
    })?;

    // 2. Get the 'required_list' array passed as an argument
    let required_list = args
        .get("required_list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            tera::Error::msg(
                "is_required filter requires 'required_list' argument, and it must be an array.",
            )
        })?;

    // 3. Look up the property name in the array
    let is_required = required_list.iter().any(|v| v.as_str() == Some(prop_name));

    // 4. Return a boolean value
    to_value(is_required)
        .map_err(|e| tera::Error::msg(format!("Failed to convert bool to Value: {}", e)))
}
