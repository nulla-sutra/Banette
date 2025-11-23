use std::collections::HashMap;
use tera::{Result, Value, to_value};

pub(crate) fn to_ue_type_filter(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
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

        // 3. Get the type string
        let type_str = schema
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("object");

        match type_str {
            "string" => "FString".to_string(),
            "integer" => {
                // Optional: check 'format' to distinguish int32/int64
                let format = schema.get("format").and_then(|f| f.as_str());
                match format {
                    Some("int64") => "int64".to_string(),
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

    let result = get_cpp_type(value);
    Ok(to_value(result)?)
}

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

/// Convert an OpenAPI path to a PascalCase function name and append the HTTP method.
///
/// Example: /v1/player/characters, method="get" -> V1PlayerCharacters_GET
pub(crate) fn path_to_func_name_filter(
    value: &Value,
    args: &HashMap<String, Value>,
) -> Result<Value> {
    let path = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("Path must be a string"))?;

    // 1. Get and uppercase the HTTP method (GET, POST, etc.)
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("path_to_func_name requires a 'method' argument"))?
        .to_uppercase();

    // 2. Remove the leading slash
    let cleaned_path = path.trim_start_matches('/');

    // 3. Split and apply PascalCase transformation
    let parts: Vec<&str> = cleaned_path.split('/').collect();
    let mut func_base_name = String::new();

    for part in parts {
        if part.is_empty() {
            continue;
        }

        let mut chars = part.chars();
        // Capitalize the first character
        if let Some(first_char) = chars.next() {
            func_base_name.push_str(&first_char.to_uppercase().to_string());
            // Append the rest
            func_base_name.push_str(chars.as_str());
        }
    }

    // 4. Combine the function name and the method
    let final_name = format!("{}_{}", func_base_name, method);

    Ok(to_value(final_name)?)
}

pub fn get_request_body_schema_filter(
    value: &Value,
    _args: &HashMap<String, Value>,
) -> Result<Value> {
    // 1. Check that the input is an object
    let req_body = value.as_object().ok_or_else(|| {
        tera::Error::msg("Input to get_body_schema must be a valid requestBody object.")
    })?;

    // 2. Get the "content" field
    let content = req_body
        .get("content")
        .ok_or_else(|| tera::Error::msg("requestBody object is missing 'content' field."))?;

    // 3. Try to find the schema for "application/json"
    if let Some(schema_obj) = content
        .get("application/json")
        .and_then(|json_media_type| json_media_type.get("schema"))
    {
        return Ok(schema_obj.clone());
    }

    // 4. Fallback: if there is no application/json, try the first available media type
    if let Some(content_map) = content.as_object() {
        if let Some((_, media_type)) = content_map.iter().next() {
            if let Some(schema_obj) = media_type.get("schema") {
                return Ok(schema_obj.clone());
            }
        }
    }

    // 5. Failure handling
    Err(tera::Error::msg(
        "Could not find a valid schema object within requestBody content (checked application/json and first available type).",
    ))
}

/// Tera filter to extract the schema from an OpenAPI response object.
///
/// The filter attempts to extract the schema in the following order:
/// 1. First, it looks for "application/json" media type
/// 2. If not found, it falls back to the first available media type
///
/// Usage in template: {{ response | get_response_schema }}
pub fn get_response_schema_filter(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    // 1. Check that the input is an object
    let response = value.as_object().ok_or_else(|| {
        tera::Error::msg("Input to get_response_schema must be a valid response object.")
    })?;

    // 2. Get the "content" field
    let content = response
        .get("content")
        .ok_or_else(|| tera::Error::msg("Response object is missing 'content' field."))?;

    // 3. Try to find the schema for "application/json"
    if let Some(schema_obj) = content
        .get("application/json")
        .and_then(|json_media_type| json_media_type.get("schema"))
    {
        return Ok(schema_obj.clone());
    }

    // 4. Fallback: if there is no application/json, try the first available media type
    if let Some(content_map) = content.as_object()
        && let Some((_, media_type)) = content_map.iter().next()
        && let Some(schema_obj) = media_type.get("schema")
    {
        return Ok(schema_obj.clone());
    }

    // 5. Failure handling
    Err(tera::Error::msg(
        "Could not find a valid schema object within response content (checked application/json and first available type).",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_response_schema_with_application_json() {
        // Create a mock response object with application/json
        let response = json!({
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "integer" },
                            "name": { "type": "string" }
                        }
                    }
                }
            }
        });

        let value = to_value(&response).unwrap();
        let result = get_response_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify the schema was extracted correctly
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "object");
        assert!(result.get("properties").is_some());
    }

    #[test]
    fn test_get_response_schema_with_fallback() {
        // Create a mock response object without application/json
        let response = json!({
            "content": {
                "text/plain": {
                    "schema": {
                        "type": "string"
                    }
                }
            }
        });

        let value = to_value(&response).unwrap();
        let result = get_response_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify the schema from the fallback media type was extracted
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "string");
    }

    #[test]
    fn test_get_response_schema_prefers_application_json() {
        // Create a mock response object with multiple media types
        let response = json!({
            "content": {
                "text/plain": {
                    "schema": {
                        "type": "string"
                    }
                },
                "application/json": {
                    "schema": {
                        "type": "object"
                    }
                }
            }
        });

        let value = to_value(&response).unwrap();
        let result = get_response_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify application/json was preferred
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "object");
    }

    #[test]
    fn test_get_response_schema_missing_content() {
        // Create a mock response object without content field
        let response = json!({
            "description": "A response without content"
        });

        let value = to_value(&response).unwrap();
        let result = get_response_schema_filter(&value, &HashMap::new());

        // Verify error message
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("missing 'content' field"));
    }

    #[test]
    fn test_get_response_schema_missing_schema() {
        // Create a mock response object with content but no schema
        let response = json!({
            "content": {
                "application/json": {
                    "example": "some example"
                }
            }
        });

        let value = to_value(&response).unwrap();
        let result = get_response_schema_filter(&value, &HashMap::new());

        // Verify error message
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Could not find a valid schema object"));
    }

    #[test]
    fn test_get_response_schema_invalid_input() {
        // Test with non-object input
        let value = to_value("not an object").unwrap();
        let result = get_response_schema_filter(&value, &HashMap::new());

        // Verify error message
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("must be a valid response object"));
    }
}
