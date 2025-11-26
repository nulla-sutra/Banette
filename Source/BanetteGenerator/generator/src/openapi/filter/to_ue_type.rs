/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

use std::collections::HashMap;
use tera::{to_value, Result, Value};

pub fn to_ue_type_filter(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    fn get_cpp_type(schema: &Value) -> String {
        // 1. Handle boolean Schema (true/false)
        if let Some(is_any) = schema.as_bool() {
            return if is_any {
                "FInstancedStruct".to_string() // Any type
            } else {
                "void*".to_string() // Impossible type
            };
        }

        // 2. Handle $ref references
        // If $ref exists, return the corresponding struct name directly; no need to recurse further
        if let Some(ref_path) = schema.get("$ref").and_then(|v| v.as_str()) {
            let struct_name = ref_path.split('/').last().unwrap_or("Unknown");
            return format!("F{}", struct_name);
        }

        // 3. Get the type string, handling nullable types (arrays with "null")
        let type_str = get_effective_type(schema);

        match type_str.as_str() {
            "string" => "FString".to_string(),
            "integer" => {
                // Check 'format' to distinguish int32/int64/uint8
                let format = schema.get("format").and_then(|f| f.as_str());
                match format {
                    Some("int64") => "int64".to_string(),
                    Some("uint") => "uint8".to_string(),
                    _ => "int32".to_string(),
                }
            }
            "number" => "float".to_string(),
            "boolean" => "bool".to_string(),
            "array" => {
                // === Recursion key point ===
                // Get the 'items' field
                if let Some(items) = schema.get("items") {
                    // Recursively call itself to get the inner type
                    let inner_type = get_cpp_type(items);
                    format!("TArray<{}>", inner_type)
                } else {
                    // If it's an array without 'items' defined, assume an array of any type
                    "TArray<FInstancedStruct>".to_string()
                }
            }
            // object or other cases
            _ => "FInstancedStruct".to_string(),
        }
    }

    /// Extracts the effective type string from the schema.
    /// Handles nullable types where `type` is an array containing a concrete type and "null".
    /// Returns the non-null concrete type, or falls back to "object" if none is found.
    fn get_effective_type(schema: &Value) -> String {
        if let Some(type_value) = schema.get("type") {
            // Handle case where type is a simple string
            if let Some(type_str) = type_value.as_str() {
                return type_str.to_string();
            }

            // Handle case where type is an array (nullable types like ["integer", "null"])
            if let Some(type_array) = type_value.as_array() {
                // Filter out "null" and get the concrete types
                let concrete_types: Vec<&str> = type_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .filter(|t| *t != "null")
                    .collect();

                // If there's exactly one concrete type, use it
                if concrete_types.len() == 1 {
                    return concrete_types[0].to_string();
                }
            }
        }

        // Default to "object" if no valid type is found
        "object".to_string()
    }

    let result = get_cpp_type(value);
    Ok(to_value(result)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tera::to_value;

    #[test]
    fn test_to_ue_type_string() {
        let schema = json!({"type": "string"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "FString");
    }

    #[test]
    fn test_to_ue_type_integer_default() {
        let schema = json!({"type": "integer"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "int32");
    }

    #[test]
    fn test_to_ue_type_integer_int32() {
        let schema = json!({"type": "integer", "format": "int32"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "int32");
    }

    #[test]
    fn test_to_ue_type_integer_int64() {
        let schema = json!({"type": "integer", "format": "int64"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "int64");
    }

    #[test]
    fn test_to_ue_type_integer_uint() {
        let schema = json!({"type": "integer", "format": "uint"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "uint8");
    }

    #[test]
    fn test_to_ue_type_number() {
        let schema = json!({"type": "number"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "float");
    }

    #[test]
    fn test_to_ue_type_boolean() {
        let schema = json!({"type": "boolean"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "bool");
    }

    #[test]
    fn test_to_ue_type_array_with_items() {
        let schema = json!({
            "type": "array",
            "items": {"type": "string"}
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "TArray<FString>");
    }

    #[test]
    fn test_to_ue_type_array_without_items() {
        let schema = json!({"type": "array"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "TArray<FInstancedStruct>");
    }

    #[test]
    fn test_to_ue_type_object() {
        let schema = json!({"type": "object"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "FInstancedStruct");
    }

    #[test]
    fn test_to_ue_type_ref() {
        let schema = json!({"$ref": "#/components/schemas/User"});
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "FUser");
    }

    #[test]
    fn test_to_ue_type_boolean_schema_true() {
        let value = to_value(true).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "FInstancedStruct");
    }

    #[test]
    fn test_to_ue_type_boolean_schema_false() {
        let value = to_value(false).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "void*");
    }

    // Nullable type tests
    #[test]
    fn test_to_ue_type_nullable_integer_int32() {
        // OpenAPI nullable type: ["integer", "null"] with format: "int32"
        let schema = json!({
            "type": ["integer", "null"],
            "format": "int32"
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "int32");
    }

    #[test]
    fn test_to_ue_type_nullable_integer_uint() {
        // OpenAPI nullable type: ["integer", "null"] with format: "uint"
        let schema = json!({
            "type": ["integer", "null"],
            "format": "uint"
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "uint8");
    }

    #[test]
    fn test_to_ue_type_nullable_integer_default() {
        // OpenAPI nullable type: ["integer", "null"] without format
        let schema = json!({
            "type": ["integer", "null"]
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "int32");
    }

    #[test]
    fn test_to_ue_type_nullable_string() {
        // OpenAPI nullable type: ["string", "null"]
        let schema = json!({
            "type": ["string", "null"]
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "FString");
    }

    #[test]
    fn test_to_ue_type_nullable_boolean() {
        // OpenAPI nullable type: ["boolean", "null"]
        let schema = json!({
            "type": ["boolean", "null"]
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "bool");
    }

    #[test]
    fn test_to_ue_type_nullable_number() {
        // OpenAPI nullable type: ["number", "null"]
        let schema = json!({
            "type": ["number", "null"]
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "float");
    }

    #[test]
    fn test_to_ue_type_nullable_array() {
        // OpenAPI nullable type: ["array", "null"]
        let schema = json!({
            "type": ["array", "null"],
            "items": {"type": "string"}
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "TArray<FString>");
    }

    #[test]
    fn test_to_ue_type_null_first_in_array() {
        // OpenAPI nullable type with null first: ["null", "integer"]
        let schema = json!({
            "type": ["null", "integer"],
            "format": "int64"
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "int64");
    }

    #[test]
    fn test_to_ue_type_multiple_non_null_types_fallback() {
        // If there are multiple non-null types, fall back to FInstancedStruct
        let schema = json!({
            "type": ["integer", "string", "null"]
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "FInstancedStruct");
    }

    #[test]
    fn test_to_ue_type_only_null_type() {
        // If only "null" is present, fall back to FInstancedStruct
        let schema = json!({
            "type": ["null"]
        });
        let value = to_value(&schema).unwrap();
        let result = to_ue_type_filter(&value, &HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "FInstancedStruct");
    }
}
