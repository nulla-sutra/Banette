/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

use std::collections::HashMap;
use tera::{to_value, Result, Value};

/// Tera filter to generate FHttpRequest chain call methods (`.With_xxx`) from OpenAPI path-item.
///
/// This filter takes a path string, HTTP method, optional parameters, and optional requestBody,
/// then generates the chained `.With_xxx` method calls for building an FHttpRequest.
///
/// FHttpRequest supports the following With_xxx methods:
/// - `.With_Url(...)` - URL address
/// - `.With_Method(...)` - HTTP method (EHttpMethod::Get, Post, Put, Delete, Patch, Head)
/// - `.With_ContentType(...)` - Content-Type (from requestBody.content)
/// - `.With_Body(...)` - Request body using ToBinary(RequestBody)
///
/// Usage in template:
/// ```tera
/// {{ path | http_request_builder(method=method, parameters=operation.parameters, request_body=operation.requestBody) }}
/// ```
///
/// Examples:
/// - `/v1/characters`, method="get" ->
///   `.With_Url(TEXT("/v1/characters")).With_Method(EHttpMethod::Get)`
/// - `/v1/characters`, method="post", requestBody with application/json ->
///   `.With_Url(TEXT("/v1/characters")).With_Method(EHttpMethod::Post).With_ContentType(TEXT("application/json")).With_Body(ToBinary(RequestBody))`
pub fn http_request_builder_filter(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    // 1. Get the path string
    let path = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("Path must be a string"))?;

    // 2. Get the HTTP method argument
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("http_request_builder requires a 'method' argument"))?;

    // 3. Get the optional parameters array
    let parameters = args.get("parameters").and_then(|v| v.as_array());

    // 4. Get the optional request_body object
    let request_body = args.get("request_body");

    // 5. Convert the HTTP method to EHttpMethod enum value
    let http_method = convert_to_http_method(method)?;

    // 6. Extract path parameters from the parameter array (where "in": "path")
    let path_params = extract_path_parameters(parameters);

    // 7. Extract query parameters from the parameter array (where "in": "query")
    let query_params = extract_query_parameters(parameters);

    // 8. Build the URL expression
    let url_expr = build_url_expression(path, &path_params, &query_params);

    // 9. Build the chain calls
    let mut chain_calls = Vec::new();

    // Add .With_Url(...)
    chain_calls.push(format!(".With_Url({})", url_expr));

    // Add .With_Method(...)
    chain_calls.push(format!(".With_Method(EHttpMethod::{})", http_method));

    // Add .With_ContentType(...) and .With_Body(...) if requestBody exists
    if let Some(body) = request_body
        && body.is_object()
    {
        if let Some(content_type) = extract_content_type(body) {
            chain_calls.push(format!(
                ".With_ContentType(TEXT(\"{}\"))",
                escape_cpp_string(&content_type)
            ));
        }
        chain_calls.push(".With_Body(ToBytes(RequestBody))".to_string());
    }

    // Join all chain calls
    let result = format!("FHttpRequest(){}", chain_calls.join(""));

    Ok(to_value(result)?)
}

/// Convert an HTTP method string to the corresponding EHttpMethod enum variant name.
///
/// Supported methods: get, post, put, delete, patch, head
/// Returns PascalCase variant name for use in C++ code.
fn convert_to_http_method(method: &str) -> Result<&'static str> {
    match method.to_lowercase().as_str() {
        "get" => Ok("Get"),
        "post" => Ok("Post"),
        "put" => Ok("Put"),
        "delete" => Ok("Delete"),
        "patch" => Ok("Patch"),
        "head" => Ok("Head"),
        _ => Err(tera::Error::msg(format!(
            "Unsupported HTTP method: '{}'. Supported methods are: get, post, put, delete, patch, head",
            method
        ))),
    }
}

/// Escape special characters in a string for use in a C++ string literal.
///
/// Escapes backslashes and double quotes to prevent code injection.
fn escape_cpp_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Extract path parameters from the OpenAPI parameters array.
///
/// Path parameters have `"in": "path"` in their definition.
/// Returns a vector of parameter names.
fn extract_path_parameters(parameters: Option<&Vec<Value>>) -> Vec<String> {
    let Some(params) = parameters else {
        return Vec::new();
    };

    params
        .iter()
        .filter_map(|param| {
            let in_type = param.get("in")?.as_str()?;
            if in_type == "path" {
                param.get("name")?.as_str().map(String::from)
            } else {
                None
            }
        })
        .collect()
}

/// Extract query parameters from the OpenAPI parameters array.
///
/// Query parameters have `"in": "query"` in their definition.
/// Returns a vector of parameter names.
fn extract_query_parameters(parameters: Option<&Vec<Value>>) -> Vec<String> {
    let Some(params) = parameters else {
        return Vec::new();
    };

    params
        .iter()
        .filter_map(|param| {
            let in_type = param.get("in")?.as_str()?;
            if in_type == "query" {
                param.get("name")?.as_str().map(String::from)
            } else {
                None
            }
        })
        .collect()
}

/// Build the URL expression for the FHttpRequest.
///
/// If there are path parameters or query parameters, use FString::Format with
/// FStringFormatNamedArguments. Otherwise, uses a simple TEXT() macro.
fn build_url_expression(path: &str, path_params: &[String], query_params: &[String]) -> String {
    let escaped_path = escape_cpp_string(path);

    // If no parameters, use simple TEXT() macro
    if path_params.is_empty() && query_params.is_empty() {
        return format!("TEXT(\"{}\")", escaped_path);
    }

    // Build the URL template with query parameters appended
    let mut url_template = escaped_path;
    if !query_params.is_empty() {
        let query_string: Vec<String> = query_params
            .iter()
            .map(|name| format!("{}={{{}}}", name, name))
            .collect();
        url_template = format!("{}?{}", url_template, query_string.join("&"));
    }

    // Collect all parameter names (path and query)
    let all_params: Vec<&String> = path_params.iter().chain(query_params.iter()).collect();

    // Build FStringFormatNamedArguments
    let args_entries: Vec<String> = all_params
        .iter()
        .map(|name| format!("{{\"{}\", {}}}", name, name))
        .collect();
    let format_args = format!("FStringFormatNamedArguments{{{}}}", args_entries.join(", "));

    format!(
        "FString::Format(TEXT(\"{}\"), {})",
        url_template, format_args
    )
}

/// Extract the Content-Type from a requestBody object.
///
/// Prefers "application/json", but falls back to the first available content type.
fn extract_content_type(request_body: &Value) -> Option<String> {
    let content = request_body.get("content")?.as_object()?;

    // Prefer application/json
    if content.contains_key("application/json") {
        return Some("application/json".to_string());
    }

    // Fallback to the first available content type
    content.keys().next().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::filter::tests::create_method_args;
    use serde_json::json;

    /// Helper function to create args with method, parameters, and request_body
    fn create_full_args(
        method: &str,
        parameters: Option<Value>,
        request_body: Option<Value>,
    ) -> HashMap<String, Value> {
        let mut args = create_method_args(method);
        if let Some(params) = parameters {
            args.insert("parameters".to_string(), params);
        }
        if let Some(body) = request_body {
            args.insert("request_body".to_string(), body);
        }
        args
    }

    // Test 1: Simple GET request (no requestBody)
    #[test]
    fn test_simple_get_request() {
        let path = json!("/v1/characters");
        let args = create_method_args("get");

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/v1/characters\")).With_Method(EHttpMethod::Get)"
        );
    }

    // Test 2: POST request with application/json requestBody
    #[test]
    fn test_post_request_with_json_body() {
        let path = json!("/v1/characters");
        let request_body = json!({
            "content": {
                "application/json": {
                    "schema": {
                        "$ref": "#/components/schemas/CreateCharacterRequest"
                    }
                }
            },
            "required": true
        });
        let args = create_full_args("post", None, Some(request_body));

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/v1/characters\")).With_Method(EHttpMethod::Post).With_ContentType(TEXT(\"application/json\")).With_Body(ToBinary(RequestBody))"
        );
    }

    // Test 3: PUT request with path parameters and requestBody
    #[test]
    fn test_put_request_with_path_params_and_body() {
        let path = json!("/v1/characters/{id}");
        let parameters = json!([
            {"in": "path", "name": "id", "required": true, "schema": {"type": "string"}}
        ]);
        let request_body = json!({
            "content": {
                "application/json": {
                    "schema": {
                        "$ref": "#/components/schemas/UpdateCharacterRequest"
                    }
                }
            }
        });
        let args = create_full_args("put", Some(parameters), Some(request_body));

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(FString::Format(TEXT(\"/v1/characters/{id}\"), FStringFormatNamedArguments{{\"id\", id}})).With_Method(EHttpMethod::Put).With_ContentType(TEXT(\"application/json\")).With_Body(ToBinary(RequestBody))"
        );
    }

    // Test 4: DELETE request with path parameters
    #[test]
    fn test_delete_request_with_path_params() {
        let path = json!("/v1/characters/{id}");
        let parameters = json!([
            {"in": "path", "name": "id", "required": true, "schema": {"type": "string"}}
        ]);
        let args = create_full_args("delete", Some(parameters), None);

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(FString::Format(TEXT(\"/v1/characters/{id}\"), FStringFormatNamedArguments{{\"id\", id}})).With_Method(EHttpMethod::Delete)"
        );
    }

    // Test 5: GET request with query parameters
    #[test]
    fn test_get_request_with_query_params() {
        let path = json!("/v1/characters");
        let parameters = json!([
            {"in": "query", "name": "shard", "schema": {"type": "string"}},
            {"in": "query", "name": "limit", "schema": {"type": "integer"}}
        ]);
        let args = create_full_args("get", Some(parameters), None);

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(FString::Format(TEXT(\"/v1/characters?shard={shard}&limit={limit}\"), FStringFormatNamedArguments{{\"shard\", shard}, {\"limit\", limit}})).With_Method(EHttpMethod::Get)"
        );
    }

    // Test 6: POST request with text/plain Content-Type
    #[test]
    fn test_post_request_with_text_plain_body() {
        let path = json!("/v1/messages");
        let request_body = json!({
            "content": {
                "text/plain": {
                    "schema": {
                        "type": "string"
                    }
                }
            }
        });
        let args = create_full_args("post", None, Some(request_body));

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/v1/messages\")).With_Method(EHttpMethod::Post).With_ContentType(TEXT(\"text/plain\")).With_Body(ToBinary(RequestBody))"
        );
    }

    // Test 7: Missing method parameter error handling
    #[test]
    fn test_missing_method_error() {
        let path = json!("/v1/characters");
        let args = HashMap::new();

        let result = http_request_builder_filter(&path, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("method"));
    }

    // Test 8: HEAD method
    #[test]
    fn test_head_method() {
        let path = json!("/health");
        let args = create_method_args("head");

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/health\")).With_Method(EHttpMethod::Head)"
        );
    }

    // Test 9: PATCH method with body
    #[test]
    fn test_patch_method_with_body() {
        let path = json!("/v1/users/{id}");
        let parameters = json!([
            {"in": "path", "name": "id", "required": true, "schema": {"type": "string"}}
        ]);
        let request_body = json!({
            "content": {
                "application/json": {
                    "schema": {
                        "$ref": "#/components/schemas/PatchUserRequest"
                    }
                }
            }
        });
        let args = create_full_args("patch", Some(parameters), Some(request_body));

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(FString::Format(TEXT(\"/v1/users/{id}\"), FStringFormatNamedArguments{{\"id\", id}})).With_Method(EHttpMethod::Patch).With_ContentType(TEXT(\"application/json\")).With_Body(ToBinary(RequestBody))"
        );
    }

    // Test 10: Case insensitive method
    #[test]
    fn test_case_insensitive_method() {
        let path = json!("/v1/data");
        let args = create_method_args("GET");

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/v1/data\")).With_Method(EHttpMethod::Get)"
        );
    }

    // Test 11: Mixed case method
    #[test]
    fn test_mixed_case_method() {
        let path = json!("/v1/data");
        let _args = create_method_args("PoSt");
        let request_body = json!({
            "content": {
                "application/json": {}
            }
        });
        let args = create_full_args("PoSt", None, Some(request_body));

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/v1/data\")).With_Method(EHttpMethod::Post).With_ContentType(TEXT(\"application/json\")).With_Body(ToBinary(RequestBody))"
        );
    }

    // Test 12: Unsupported method error
    #[test]
    fn test_unsupported_method_error() {
        let path = json!("/v1/data");
        let args = create_method_args("options");

        let result = http_request_builder_filter(&path, &args);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported HTTP method"));
        assert!(error_msg.contains("options"));
    }

    // Test 13: Invalid path type error
    #[test]
    fn test_invalid_path_type_error() {
        let path = json!(123);
        let args = create_method_args("get");

        let result = http_request_builder_filter(&path, &args);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Path must be a string")
        );
    }

    // Test 14: Path with both path and query parameters
    #[test]
    fn test_path_and_query_params_combined() {
        let path = json!("/v1/users/{user_id}/posts/{post_id}");
        let parameters = json!([
            {"in": "path", "name": "user_id", "required": true},
            {"in": "path", "name": "post_id", "required": true},
            {"in": "query", "name": "include_comments"},
            {"in": "query", "name": "limit"}
        ]);
        let args = create_full_args("get", Some(parameters), None);

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(FString::Format(TEXT(\"/v1/users/{user_id}/posts/{post_id}?include_comments={include_comments}&limit={limit}\"), FStringFormatNamedArguments{{\"user_id\", user_id}, {\"post_id\", post_id}, {\"include_comments\", include_comments}, {\"limit\", limit}})).With_Method(EHttpMethod::Get)"
        );
    }

    // Test 15: Empty path
    #[test]
    fn test_empty_path() {
        let path = json!("");
        let args = create_method_args("get");

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"\")).With_Method(EHttpMethod::Get)"
        );
    }

    // Test 16: Root path
    #[test]
    fn test_root_path() {
        let path = json!("/");
        let args = create_method_args("get");

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/\")).With_Method(EHttpMethod::Get)"
        );
    }

    // Test 17: RequestBody with null value is ignored
    #[test]
    fn test_null_request_body_ignored() {
        let path = json!("/v1/data");
        let args = create_full_args("post", None, Some(json!(null)));

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/v1/data\")).With_Method(EHttpMethod::Post)"
        );
    }

    // Test 18: Extract content type prefers application/json
    #[test]
    fn test_extract_content_type_prefers_json() {
        let request_body = json!({
            "content": {
                "text/plain": {},
                "application/json": {},
                "application/xml": {}
            }
        });

        let content_type = extract_content_type(&request_body);
        assert_eq!(content_type, Some("application/json".to_string()));
    }

    // Test 19: Extract content type falls back to the first available
    #[test]
    fn test_extract_content_type_fallback() {
        let request_body = json!({
            "content": {
                "text/plain": {}
            }
        });

        let content_type = extract_content_type(&request_body);
        assert_eq!(content_type, Some("text/plain".to_string()));
    }

    // Test 20: Multiple content types - prefers application/json
    #[test]
    fn test_multiple_content_types_prefers_json() {
        let path = json!("/v1/upload");
        let request_body = json!({
            "content": {
                "multipart/form-data": {},
                "application/json": {}
            }
        });
        let args = create_full_args("post", None, Some(request_body));

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/v1/upload\")).With_Method(EHttpMethod::Post).With_ContentType(TEXT(\"application/json\")).With_Body(ToBinary(RequestBody))"
        );
    }

    // Test 21: Special characters in a path are escaped
    #[test]
    fn test_special_characters_in_path() {
        let path = json!("/api/path\"with\"quotes");
        let args = create_method_args("get");

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/api/path\\\"with\\\"quotes\")).With_Method(EHttpMethod::Get)"
        );
    }

    // Test 22: Backslash in a path is escaped
    #[test]
    fn test_backslash_in_path() {
        let path = json!("/api/path\\with\\backslash");
        let args = create_method_args("post");

        let result = http_request_builder_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/api/path\\\\with\\\\backslash\")).With_Method(EHttpMethod::Post)"
        );
    }

    // Test 23: Complex example from a problem statement
    #[test]
    fn test_problem_statement_example_post() {
        let path = json!("/v1/characters");
        let request_body = json!({
            "content": {
                "application/json": {
                    "schema": {
                        "$ref": "#/components/schemas/CreateCharacterRequest"
                    }
                }
            },
            "required": true
        });
        let args = create_full_args("post", None, Some(request_body));

        let result = http_request_builder_filter(&path, &args).unwrap();
        // Expected output from a problem statement:
        // .With_Url(TEXT("/v1/characters"))
        // .With_Method(EHttpMethod::Post)
        // .With_ContentType(TEXT("application/json"))
        // .With_Body(ToBinary(RequestBody))
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/v1/characters\")).With_Method(EHttpMethod::Post).With_ContentType(TEXT(\"application/json\")).With_Body(ToBinary(RequestBody))"
        );
    }

    // Test 24: GET request without requestBody (from a problem statement)
    #[test]
    fn test_problem_statement_example_get() {
        let path = json!("/v1/characters");
        let args = create_method_args("get");

        let result = http_request_builder_filter(&path, &args).unwrap();
        // Expected output from a problem statement:
        // .With_Url(TEXT("/v1/characters"))
        // .With_Method(EHttpMethod::Get)
        assert_eq!(
            result.as_str().unwrap(),
            ".With_Url(TEXT(\"/v1/characters\")).With_Method(EHttpMethod::Get)"
        );
    }
}
