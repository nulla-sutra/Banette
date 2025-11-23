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
/// Handles path parameters (enclosed in `{}`) by converting them to PascalCase.
///
/// Examples:
/// - `/v1/player/characters`, method="get" -> `V1PlayerCharacters_GET`
/// - `/character/{id}`, method="get" -> `CharacterId_GET`
/// - `/user/{user_id}/posts`, method="get" -> `UserUserIdPosts_GET`
/// - `/api/{resource_id}/sub/{sub_id}`, method="post" -> `ApiResourceIdSubSubId_POST`
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

        // Check if this part is a path parameter (enclosed in {})
        let processed_part = if part.starts_with('{') && part.ends_with('}') {
            // Remove the braces
            let param_name = &part[1..part.len() - 1];

            // Validate that the parameter name is not empty
            if param_name.is_empty() {
                // Skip empty parameters like {}
                continue;
            }

            // Convert parameter name to PascalCase
            convert_to_pascal_case(param_name)
        } else {
            // Regular path segment - just capitalize the first character
            let mut chars = part.chars();
            if let Some(first_char) = chars.next() {
                let mut result = first_char.to_uppercase().to_string();
                result.push_str(chars.as_str());
                result
            } else {
                String::new()
            }
        };

        func_base_name.push_str(&processed_part);
    }

    // 4. Combine the function name and the method
    let final_name = format!("{}_{}", func_base_name, method);

    Ok(to_value(final_name)?)
}

/// Convert a string to PascalCase.
///
/// Handles underscores, hyphens, and camelCase/snake_case inputs.
/// Returns an empty string if input is empty.
///
/// Examples:
/// - `id` -> `Id`
/// - `user_id` -> `UserId`
/// - `resource-name` -> `ResourceName`
/// - `userId` -> `UserId`
fn convert_to_pascal_case(input: &str) -> String {
    // Handle empty input
    if input.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut capitalize_next = true;

    for ch in input.chars() {
        if ch == '_' || ch == '-' {
            // Treat underscore and hyphen as word separators
            capitalize_next = true;
        } else if ch.is_uppercase() {
            // If we encounter an uppercase letter in camelCase, keep it
            result.push(ch);
            capitalize_next = false;
        } else if capitalize_next {
            // Capitalize this character
            result.push_str(&ch.to_uppercase().to_string());
            capitalize_next = false;
        } else {
            // Keep the character as-is
            result.push(ch);
        }
    }

    result
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

pub fn get_response_schema_filter(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Helper function to create args for path_to_func_name_filter
    fn create_method_args(method: &str) -> HashMap<String, Value> {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value(method).unwrap());
        args
    }

    #[test]
    fn test_path_to_func_name_simple_path() {
        let path = json!("/v1/player/characters");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "V1PlayerCharacters_GET");
    }

    #[test]
    fn test_path_to_func_name_with_single_parameter() {
        let path = json!("/character/{id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "CharacterId_GET");
    }

    #[test]
    fn test_path_to_func_name_with_snake_case_parameter() {
        let path = json!("/user/{user_id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "UserUserId_GET");
    }

    #[test]
    fn test_path_to_func_name_with_multiple_parameters() {
        let path = json!("/user/{user_id}/posts/{post_id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "UserUserIdPostsPostId_GET");
    }

    #[test]
    fn test_path_to_func_name_with_hyphenated_parameter() {
        let path = json!("/resource/{resource-id}");
        let args = create_method_args("delete");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "ResourceResourceId_DELETE");
    }

    #[test]
    fn test_path_to_func_name_complex_path() {
        let path = json!("/api/v2/{resource_id}/sub/{sub_id}/details");
        let args = create_method_args("post");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "ApiV2ResourceIdSubSubIdDetails_POST"
        );
    }

    #[test]
    fn test_path_to_func_name_different_methods() {
        let path = json!("/items/{id}");

        // Test with GET
        let args_get = create_method_args("get");
        let result_get = path_to_func_name_filter(&path, &args_get).unwrap();
        assert_eq!(result_get.as_str().unwrap(), "ItemsId_GET");

        // Test with POST
        let args_post = create_method_args("post");
        let result_post = path_to_func_name_filter(&path, &args_post).unwrap();
        assert_eq!(result_post.as_str().unwrap(), "ItemsId_POST");

        // Test with PUT
        let args_put = create_method_args("put");
        let result_put = path_to_func_name_filter(&path, &args_put).unwrap();
        assert_eq!(result_put.as_str().unwrap(), "ItemsId_PUT");

        // Test with DELETE
        let args_delete = create_method_args("delete");
        let result_delete = path_to_func_name_filter(&path, &args_delete).unwrap();
        assert_eq!(result_delete.as_str().unwrap(), "ItemsId_DELETE");
    }

    #[test]
    fn test_path_to_func_name_with_camel_case_parameter() {
        // Test that camelCase parameters are converted correctly
        let path = json!("/resource/{userId}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "ResourceUserId_GET");
    }

    #[test]
    fn test_convert_to_pascal_case() {
        assert_eq!(convert_to_pascal_case("id"), "Id");
        assert_eq!(convert_to_pascal_case("user_id"), "UserId");
        assert_eq!(convert_to_pascal_case("resource-name"), "ResourceName");
        assert_eq!(convert_to_pascal_case("userId"), "UserId");
        assert_eq!(convert_to_pascal_case("post_author_id"), "PostAuthorId");
        assert_eq!(convert_to_pascal_case("userId_name"), "UserIdName");
        assert_eq!(convert_to_pascal_case("resource-type-id"), "ResourceTypeId");
        assert_eq!(convert_to_pascal_case("mixed_case-value"), "MixedCaseValue");
    }

    #[test]
    fn test_path_to_func_name_edge_cases() {
        // Test empty path segments (shouldn't happen in real APIs, but good to handle)
        let path = json!("/api//resource/{id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "ApiResourceId_GET");
    }

    #[test]
    fn test_path_to_func_name_only_parameter() {
        let path = json!("/{id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "Id_GET");
    }

    #[test]
    fn test_path_to_func_name_missing_method() {
        let path = json!("/users");
        let args = HashMap::new(); // No method provided

        let result = path_to_func_name_filter(&path, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("method"));
    }

    #[test]
    fn test_path_to_func_name_invalid_path_type() {
        let path = json!(123); // Not a string
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Path must be a string")
        );
    }

    #[test]
    fn test_path_to_func_name_empty_braces() {
        // Test that empty braces {} are handled gracefully
        let path = json!("/api/{}/resource");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        // Empty braces should be skipped
        assert_eq!(result.as_str().unwrap(), "ApiResource_GET");
    }

    #[test]
    fn test_convert_to_pascal_case_empty_string() {
        // Test that empty string returns empty string
        assert_eq!(convert_to_pascal_case(""), "");
    }

    #[test]
    fn test_convert_to_pascal_case_only_separators() {
        // Test strings with only separators
        assert_eq!(convert_to_pascal_case("___"), "");
        assert_eq!(convert_to_pascal_case("---"), "");
        assert_eq!(convert_to_pascal_case("_-_"), "");
    }
}
