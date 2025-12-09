/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

use std::collections::HashMap;
use tera::{to_value, Result, Value};

/// Convert an OpenAPI path to a PascalCase function name with the HTTP method prefix.
///
/// Handles path parameters (enclosed in `{}`) by converting them to PascalCase and grouping them with the "By_" prefix.
///
/// Examples:
/// - `/v1/player/characters`, method="get" -> `GET_V1_Player_Characters`
/// - `/character/{id}`, method="get" -> `GET_Character_By_Id`
/// - `/user/{user_id}/posts`, method="get" -> `GET_User_Posts_By_UserId`
/// - `/api/{resource_id}/sub/{sub_id}`, method="post" -> `POST_Api_Sub_By_ResourceId_SubId`
pub fn path_to_func_name_filter(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
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
    let mut regular_segments = Vec::new();
    let mut parameters = Vec::new();

    for part in cleaned_path.split('/') {
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
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            // Keep the character as-is
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::tests::create_method_args;
    use serde_json::json;

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
}
