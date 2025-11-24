use std::collections::HashMap;
use tera::{Result, Value, to_value};

/// Successful HTTP status codes to prioritize when extracting response schemas
const SUCCESS_STATUS_CODES: &[&str] = &["200", "201", "202", "203", "204"];

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

/// Convert an OpenAPI path to a PascalCase function name with the HTTP method prefix.
///
/// Handles path parameters (enclosed in `{}`) by converting them to PascalCase and grouping them with the "By_" prefix.
///
/// Examples:
/// - `/v1/player/characters`, method="get" -> `GET_V1_Player_Characters`
/// - `/character/{id}`, method="get" -> `GET_Character_By_Id`
/// - `/user/{user_id}/posts`, method="get" -> `GET_User_Posts_By_UserId`
/// - `/api/{resource_id}/sub/{sub_id}`, method="post" -> `POST_Api_Sub_By_ResourceId_SubId`
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

    // 3. Split and separate into regular segments and parameters
    let parts: Vec<&str> = cleaned_path.split('/').collect();
    let mut regular_segments = Vec::new();
    let mut parameters = Vec::new();

    for part in parts {
        if part.is_empty() {
            continue;
        }

        // Check if this part is a path parameter (enclosed in {})
        if part.starts_with('{') && part.ends_with('}') {
            // Remove the braces
            let param_name = &part[1..part.len() - 1];

            // Validate that the parameter name is not empty
            if param_name.is_empty() {
                // Skip empty parameters like {}
                continue;
            }

            // Convert parameter name to PascalCase and add to a parameter list
            parameters.push(convert_to_pascal_case(param_name));
        } else {
            // Regular path segment - convert to PascalCase for consistency
            regular_segments.push(convert_to_pascal_case(part));
        }
    }

    // 4. Build the function name: METHOD_Segments_By_Parameters
    let mut func_name = method.clone();

    // Add regular segments separated by underscores
    if !regular_segments.is_empty() {
        func_name.push('_');
        func_name.push_str(&regular_segments.join("_"));
    }

    // Add parameters with the "By_" prefix
    if !parameters.is_empty() {
        func_name.push_str("_By_");
        // All parameters: By_Param1_Param2_Param3
        func_name.push_str(&parameters.join("_"));
    }

    Ok(to_value(func_name)?)
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

pub fn request_body_schema_filter(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
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

/// Tera filter to transform an array of tags into a pipe-separated string.
///
/// This filter takes an array of strings (tags) and joins them with a pipe (`|`) delimiter.
/// For example: `["Character", "Inventory"]` becomes `"Character|Inventory"`.
///
/// Usage in the template: {{ operation.tags | tags_to_pipe_separated }}
pub fn tags_to_pipe_separated_filter(
    value: &Value,
    _args: &HashMap<String, Value>,
) -> Result<Value> {
    // 1. Check if the input is an array
    let tags_array = value.as_array().ok_or_else(|| {
        tera::Error::msg("tags_to_pipe_separated filter expects an array of strings as input.")
    })?;

    // 2. Convert array elements to strings and validate
    let mut tag_strings = Vec::new();
    for (idx, tag) in tags_array.iter().enumerate() {
        let tag_str = tag.as_str().ok_or_else(|| {
            tera::Error::msg(format!(
                "tags_to_pipe_separated filter expects all elements to be strings. Element at index {} is not a string.",
                idx
            ))
        })?;
        tag_strings.push(tag_str);
    }

    // 3. Join with pipe delimiter
    let result = tag_strings.join("|");

    // 4. Return as Value
    to_value(result)
        .map_err(|e| tera::Error::msg(format!("Failed to convert string to Value: {}", e)))
}

/// Tera filter to extract the schema from an OpenAPI responses object.
///
/// This filter handles the OpenAPI `responses` structure which contains status codes
/// as keys (e.g., "200", "201", "404"). It attempts to extract the schema in the
/// following order:
/// 1. Looks for successful response status codes (200, 201, 202, 203, 204)
/// 2. Falls back to the first available response
/// 3. From the selected response, extracts schema preferring "application/json"
/// 4. If not found, use the first available media type
///
/// Usage in the template: {{ operation.responses | response_body_schema | to_ue_type }}
pub fn response_body_schema_filter(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    // 1. Check that the input is an object (response object)
    let responses = value.as_object().ok_or_else(|| {
        tera::Error::msg("Input to response_body_schema must be a valid responses object.")
    })?;

    // 2. Try to find a successful response or use the first available one
    let response = SUCCESS_STATUS_CODES
        .iter()
        .find_map(|code| responses.get(*code))
        .or_else(|| responses.values().next())
        .ok_or_else(|| tera::Error::msg("Responses object is empty."))?;

    // 4. Get the "content" field from the selected response
    let content = response
        .get("content")
        .ok_or_else(|| tera::Error::msg("Response object is missing 'content' field."))?;

    // 5. Try to find the schema for "application/json"
    if let Some(schema_obj) = content
        .get("application/json")
        .and_then(|json_media_type| json_media_type.get("schema"))
    {
        return Ok(schema_obj.clone());
    }

    // 6. Fallback: if there is no application/json, try the first available media type
    if let Some(content_map) = content.as_object()
        && let Some((_, media_type)) = content_map.iter().next()
        && let Some(schema_obj) = media_type.get("schema")
    {
        return Ok(schema_obj.clone());
    }

    // 7. Failure handling
    Err(tera::Error::msg(
        "Could not find a valid schema object within responses content (checked application/json and first available type).",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_response_body_schema_with_200_status() {
        // Create a mock responses object with "200" status code
        let responses = json!({
            "200": {
                "description": "Successful response",
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
            }
        });

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify the schema was extracted correctly
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "object");
        assert!(result.get("properties").is_some());
    }

    #[test]
    fn test_response_body_schema_with_array_and_ref() {
        // Test the exact example from the problem statement
        let responses = json!({
            "200": {
                "description": "",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/CharacterResponse"
                            }
                        }
                    }
                }
            }
        });

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify the schema was extracted correctly
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "array");
        assert!(result.get("items").is_some());
        assert_eq!(
            result
                .get("items")
                .unwrap()
                .get("$ref")
                .unwrap()
                .as_str()
                .unwrap(),
            "#/components/schemas/CharacterResponse"
        );
    }

    #[test]
    fn test_response_body_schema_prefers_200_over_404() {
        // Create a mock responses object with multiple status codes
        let responses = json!({
            "404": {
                "description": "Not found",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "error": { "type": "string" }
                            }
                        }
                    }
                }
            },
            "200": {
                "description": "Success",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "array"
                        }
                    }
                }
            }
        });

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify 200 response was preferred over 404
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "array");
    }

    #[test]
    fn test_response_body_schema_with_201_status() {
        // Test with 201 Created status
        let responses = json!({
            "201": {
                "description": "Created",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" }
                            }
                        }
                    }
                }
            }
        });

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify the schema was extracted correctly
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "object");
    }

    #[test]
    fn test_response_body_schema_fallback_to_first_status() {
        // Test with non-standard status code
        let responses = json!({
            "418": {
                "description": "I'm a teapot",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "string"
                        }
                    }
                }
            }
        });

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify the schema from the fallback status code was extracted
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "string");
    }

    #[test]
    fn test_response_body_schema_with_text_plain_fallback() {
        // Test with a non-JSON content type
        let responses = json!({
            "200": {
                "description": "Success",
                "content": {
                    "text/plain": {
                        "schema": {
                            "type": "string"
                        }
                    }
                }
            }
        });

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify the schema from the fallback media type was extracted
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "string");
    }

    #[test]
    fn test_response_body_schema_prefers_application_json() {
        // Test that application/json is preferred over other content types
        let responses = json!({
            "200": {
                "description": "Success",
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
            }
        });

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new()).unwrap();

        // Verify application/json was preferred
        assert_eq!(result.get("type").unwrap().as_str().unwrap(), "object");
    }

    #[test]
    fn test_response_body_schema_empty_responses() {
        // Test with empty responses object
        let responses = json!({});

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new());

        // Verify error message
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Responses object is empty"));
    }

    #[test]
    fn test_response_body_schema_missing_content() {
        // Test with response missing content field
        let responses = json!({
            "200": {
                "description": "A response without content"
            }
        });

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new());

        // Verify error message
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("missing 'content' field"));
    }

    #[test]
    fn test_response_body_schema_missing_schema() {
        // Test with content but no schema
        let responses = json!({
            "200": {
                "description": "Success",
                "content": {
                    "application/json": {
                        "example": "some example"
                    }
                }
            }
        });

        let value = to_value(&responses).unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new());

        // Verify error message
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Could not find a valid schema object"));
    }

    #[test]
    fn test_response_body_schema_invalid_input() {
        // Test with non-object input
        let value = to_value("not an object").unwrap();
        let result = response_body_schema_filter(&value, &HashMap::new());

        // Verify error message
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("must be a valid responses object"));
    }

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
        assert_eq!(result.as_str().unwrap(), "GET_V1_Player_Characters");
    }

    #[test]
    fn test_path_to_func_name_with_single_parameter() {
        let path = json!("/character/{id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "GET_Character_By_Id");
    }

    #[test]
    fn test_path_to_func_name_with_snake_case_parameter() {
        let path = json!("/user/{user_id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "GET_User_By_UserId");
    }

    #[test]
    fn test_path_to_func_name_with_multiple_parameters() {
        let path = json!("/user/{user_id}/posts/{post_id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "GET_User_Posts_By_UserId_PostId");
    }

    #[test]
    fn test_path_to_func_name_with_hyphenated_parameter() {
        let path = json!("/resource/{resource-id}");
        let args = create_method_args("delete");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "DELETE_Resource_By_ResourceId");
    }

    #[test]
    fn test_path_to_func_name_complex_path() {
        let path = json!("/api/v2/{resource_id}/sub/{sub_id}/details");
        let args = create_method_args("post");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "POST_Api_V2_Sub_Details_By_ResourceId_SubId"
        );
    }

    #[test]
    fn test_path_to_func_name_different_methods() {
        let path = json!("/items/{id}");

        // Test with GET
        let args_get = create_method_args("get");
        let result_get = path_to_func_name_filter(&path, &args_get).unwrap();
        assert_eq!(result_get.as_str().unwrap(), "GET_Items_By_Id");

        // Test with POST
        let args_post = create_method_args("post");
        let result_post = path_to_func_name_filter(&path, &args_post).unwrap();
        assert_eq!(result_post.as_str().unwrap(), "POST_Items_By_Id");

        // Test with PUT
        let args_put = create_method_args("put");
        let result_put = path_to_func_name_filter(&path, &args_put).unwrap();
        assert_eq!(result_put.as_str().unwrap(), "PUT_Items_By_Id");

        // Test with DELETE
        let args_delete = create_method_args("delete");
        let result_delete = path_to_func_name_filter(&path, &args_delete).unwrap();
        assert_eq!(result_delete.as_str().unwrap(), "DELETE_Items_By_Id");
    }

    #[test]
    fn test_path_to_func_name_with_camel_case_parameter() {
        // Test that camelCase parameters are converted correctly
        let path = json!("/resource/{userId}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "GET_Resource_By_UserId");
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
        assert_eq!(result.as_str().unwrap(), "GET_Api_Resource_By_Id");
    }

    #[test]
    fn test_path_to_func_name_only_parameter() {
        let path = json!("/{id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "GET_By_Id");
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
        assert_eq!(result.as_str().unwrap(), "GET_Api_Resource");
    }

    #[test]
    fn test_convert_to_pascal_case_empty_string() {
        // Test that an empty string returns empty string
        assert_eq!(convert_to_pascal_case(""), "");
    }

    #[test]
    fn test_convert_to_pascal_case_only_separators() {
        // Test strings with only separators
        assert_eq!(convert_to_pascal_case("___"), "");
        assert_eq!(convert_to_pascal_case("---"), "");
        assert_eq!(convert_to_pascal_case("_-_"), "");
    }

    /// Tests for the specific examples from the problem statement
    #[test]
    fn test_path_to_func_name_problem_statement_example_1() {
        let path = json!("/v1/player/characters");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "GET_V1_Player_Characters");
    }

    #[test]
    fn test_path_to_func_name_problem_statement_example_2() {
        let path = json!("/character/{id}");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "GET_Character_By_Id");
    }

    #[test]
    fn test_path_to_func_name_problem_statement_example_3() {
        let path = json!("/user/{user_id}/posts");
        let args = create_method_args("get");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "GET_User_Posts_By_UserId");
    }

    #[test]
    fn test_path_to_func_name_problem_statement_example_4() {
        let path = json!("/api/{resource_id}/sub/{sub_id}");
        let args = create_method_args("post");

        let result = path_to_func_name_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "POST_Api_Sub_By_ResourceId_SubId");
    }

    /// Integration test: Verify the complete pipeline from responses -> schema -> UE type
    #[test]
    fn test_responses_to_ue_type_pipeline() {
        // Test the exact example from the problem statement
        let responses = json!({
            "200": {
                "description": "",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/CharacterResponse"
                            }
                        }
                    }
                }
            }
        });

        // Step 1: Extract schema from responses
        let responses_value = to_value(&responses).unwrap();
        let schema = response_body_schema_filter(&responses_value, &HashMap::new()).unwrap();

        // Verify schema extraction
        assert_eq!(schema.get("type").unwrap().as_str().unwrap(), "array");
        assert!(schema.get("items").is_some());

        // Step 2: Convert schema to UE type
        let ue_type = to_ue_type_filter(&schema, &HashMap::new()).unwrap();

        // Verify the final UE type is correct: TArray<FCharacterResponse>
        assert_eq!(ue_type.as_str().unwrap(), "TArray<FCharacterResponse>");
    }

    /// Integration test: Verify simple object response conversion
    #[test]
    fn test_responses_to_ue_type_simple_object() {
        let responses = json!({
            "200": {
                "description": "User response",
                "content": {
                    "application/json": {
                        "schema": {
                            "$ref": "#/components/schemas/User"
                        }
                    }
                }
            }
        });

        let responses_value = to_value(&responses).unwrap();
        let schema = response_body_schema_filter(&responses_value, &HashMap::new()).unwrap();
        let ue_type = to_ue_type_filter(&schema, &HashMap::new()).unwrap();

        assert_eq!(ue_type.as_str().unwrap(), "FUser");
    }

    /// Integration test: Verify primitive type response conversion
    #[test]
    fn test_responses_to_ue_type_primitive() {
        let responses = json!({
            "200": {
                "description": "String response",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "string"
                        }
                    }
                }
            }
        });

        let responses_value = to_value(&responses).unwrap();
        let schema = response_body_schema_filter(&responses_value, &HashMap::new()).unwrap();
        let ue_type = to_ue_type_filter(&schema, &HashMap::new()).unwrap();

        assert_eq!(ue_type.as_str().unwrap(), "FString");
    }

    /// Tests for tags_to_pipe_separated_filter
    #[test]
    fn test_tags_to_pipe_separated_multiple_tags() {
        // Test with multiple tags
        let tags = json!(["Character", "Inventory"]);
        let value = to_value(&tags).unwrap();
        let result = tags_to_pipe_separated_filter(&value, &HashMap::new()).unwrap();

        assert_eq!(result.as_str().unwrap(), "Character|Inventory");
    }

    #[test]
    fn test_tags_to_pipe_separated_single_tag() {
        // Test with a single tag
        let tags = json!(["Character"]);
        let value = to_value(&tags).unwrap();
        let result = tags_to_pipe_separated_filter(&value, &HashMap::new()).unwrap();

        assert_eq!(result.as_str().unwrap(), "Character");
    }

    #[test]
    fn test_tags_to_pipe_separated_empty_tags() {
        // Test with an empty array
        let tags = json!([]);
        let value = to_value(&tags).unwrap();
        let result = tags_to_pipe_separated_filter(&value, &HashMap::new()).unwrap();

        assert_eq!(result.as_str().unwrap(), "");
    }

    #[test]
    fn test_tags_to_pipe_separated_three_tags() {
        // Test with three tags
        let tags = json!(["Character", "Inventory", "Player"]);
        let value = to_value(&tags).unwrap();
        let result = tags_to_pipe_separated_filter(&value, &HashMap::new()).unwrap();

        assert_eq!(result.as_str().unwrap(), "Character|Inventory|Player");
    }

    #[test]
    fn test_tags_to_pipe_separated_invalid_input_not_array() {
        // Test with non-array input
        let value = to_value("not an array").unwrap();
        let result = tags_to_pipe_separated_filter(&value, &HashMap::new());

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("expects an array"));
    }

    #[test]
    fn test_tags_to_pipe_separated_invalid_input_non_string_element() {
        // Test with array containing non-string elements
        let tags = json!(["Character", 123, "Inventory"]);
        let value = to_value(&tags).unwrap();
        let result = tags_to_pipe_separated_filter(&value, &HashMap::new());

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("not a string"));
        assert!(error_msg.contains("index 1"));
    }

    #[test]
    fn test_tags_to_pipe_separated_with_special_characters() {
        // Test with tags containing special characters
        let tags = json!(["Character-API", "Inventory.Service", "Player/Data"]);
        let value = to_value(&tags).unwrap();
        let result = tags_to_pipe_separated_filter(&value, &HashMap::new()).unwrap();

        assert_eq!(
            result.as_str().unwrap(),
            "Character-API|Inventory.Service|Player/Data"
        );
    }
}
