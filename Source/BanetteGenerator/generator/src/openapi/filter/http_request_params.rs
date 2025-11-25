/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

use std::collections::HashMap;
use tera::{to_value, Result, Value};

/// Tera filter to assemble FHttpRequest constructor parameters from a path-item.
///
/// This filter takes a path string, HTTP method, and optional parameters, then generates
/// the parameters needed to construct an FHttpRequest in Unreal Engine C++.
///
/// FHttpRequest has the following fields:
/// - Url: FString - The absolute URL to call
/// - Method: EHttpMethod - The HTTP verb (Get, Post, Put, Delete, Patch, Head)
///
/// Usage in the template: {{ path | http_request_params(method=method, parameters=operation.parameters) }}
///
/// Examples:
/// - `/v1/player/characters`, method="get" -> `TEXT("/v1/player/characters"), EHttpMethod::Get`
/// - `/character/{id}`, method="post" -> `FString::Format(TEXT("/character/{id}"), FStringFormatNamedArguments{{"id", id}}), EHttpMethod::Post`
/// - `/v1/player/characters`, method="get", query params -> `FString::Format(TEXT("/v1/player/characters?shard={shard}"), FStringFormatNamedArguments{{"shard", shard}}), EHttpMethod::Get`
pub fn http_request_params_filter(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    // 1. Get the path string
    let path = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("Path must be a string"))?;

    // 2. Get the HTTP method argument
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("http_request_params requires a 'method' argument"))?;

    // 3. Get the optional parameters array
    let parameters = args.get("parameters").and_then(|v| v.as_array());

    // 4. Convert the HTTP method to EHttpMethod enum value
    let http_method = convert_to_http_method(method)?;

    // 5. Extract path parameters from the parameter array (where "in": "path")
    let path_params = extract_path_parameters(parameters);

    // 6. Extract query parameters from the parameter array (where "in": "query")
    let query_params = extract_query_parameters(parameters);

    // 7. Build the URL expression
    let url_expr = build_url_expression(path, &path_params, &query_params);

    // 8. Build the constructor parameters string
    let params = format!("{}, EHttpMethod::{}", url_expr, http_method);

    Ok(to_value(params)?)
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

/// Build the URL expression for the FHttpRequest constructor.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::filter::tests::create_method_args;
    use serde_json::json;

    /// Helper function to create args with method and parameters
    fn create_args_with_params(method: &str, parameters: Option<Value>) -> HashMap<String, Value> {
        let mut args = create_method_args(method);
        if let Some(params) = parameters {
            args.insert("parameters".to_string(), params);
        }
        args
    }

    #[test]
    fn test_http_request_params_simple_get() {
        let path = json!("/v1/player/characters");
        let args = create_method_args("get");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/v1/player/characters\"), EHttpMethod::Get"
        );
    }

    #[test]
    fn test_http_request_params_with_path_parameter() {
        let path = json!("/character/{id}");
        let params = json!([
            {"in": "path", "name": "id", "required": true, "schema": {"type": "string"}}
        ]);
        let args = create_args_with_params("post", Some(params));

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "FString::Format(TEXT(\"/character/{id}\"), FStringFormatNamedArguments{{\"id\", id}}), EHttpMethod::Post"
        );
    }

    #[test]
    fn test_http_request_params_put_method() {
        let path = json!("/api/resource/{id}");
        let params = json!([
            {"in": "path", "name": "id", "required": true, "schema": {"type": "string"}}
        ]);
        let args = create_args_with_params("put", Some(params));

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "FString::Format(TEXT(\"/api/resource/{id}\"), FStringFormatNamedArguments{{\"id\", id}}), EHttpMethod::Put"
        );
    }

    #[test]
    fn test_http_request_params_delete_method() {
        let path = json!("/items/{item_id}");
        let params = json!([
            {"in": "path", "name": "item_id", "required": true, "schema": {"type": "string"}}
        ]);
        let args = create_args_with_params("delete", Some(params));

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "FString::Format(TEXT(\"/items/{item_id}\"), FStringFormatNamedArguments{{\"item_id\", item_id}}), EHttpMethod::Delete"
        );
    }

    #[test]
    fn test_http_request_params_patch_method() {
        let path = json!("/user/{user_id}/profile");
        let params = json!([
            {"in": "path", "name": "user_id", "required": true, "schema": {"type": "string"}}
        ]);
        let args = create_args_with_params("patch", Some(params));

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "FString::Format(TEXT(\"/user/{user_id}/profile\"), FStringFormatNamedArguments{{\"user_id\", user_id}}), EHttpMethod::Patch"
        );
    }

    #[test]
    fn test_http_request_params_head_method() {
        let path = json!("/health");
        let args = create_method_args("head");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/health\"), EHttpMethod::Head"
        );
    }

    #[test]
    fn test_http_request_params_uppercase_method() {
        let path = json!("/api/data");
        let args = create_method_args("GET");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/api/data\"), EHttpMethod::Get"
        );
    }

    #[test]
    fn test_http_request_params_mixed_case_method() {
        let path = json!("/api/data");
        let args = create_method_args("PoSt");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/api/data\"), EHttpMethod::Post"
        );
    }

    #[test]
    fn test_http_request_params_complex_path() {
        let path = json!("/api/v2/{resource_id}/sub/{sub_id}/details");
        let params = json!([
            {"in": "path", "name": "resource_id", "required": true, "schema": {"type": "string"}},
            {"in": "path", "name": "sub_id", "required": true, "schema": {"type": "string"}}
        ]);
        let args = create_args_with_params("get", Some(params));

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "FString::Format(TEXT(\"/api/v2/{resource_id}/sub/{sub_id}/details\"), FStringFormatNamedArguments{{\"resource_id\", resource_id}, {\"sub_id\", sub_id}}), EHttpMethod::Get"
        );
    }

    #[test]
    fn test_http_request_params_missing_method() {
        let path = json!("/users");
        let args = HashMap::new();

        let result = http_request_params_filter(&path, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("method"));
    }

    #[test]
    fn test_http_request_params_invalid_path_type() {
        let path = json!(123);
        let args = create_method_args("get");

        let result = http_request_params_filter(&path, &args);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Path must be a string")
        );
    }

    #[test]
    fn test_http_request_params_unsupported_method() {
        let path = json!("/api/resource");
        let args = create_method_args("options");

        let result = http_request_params_filter(&path, &args);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported HTTP method"));
        assert!(error_msg.contains("options"));
    }

    #[test]
    fn test_http_request_params_root_path() {
        let path = json!("/");
        let args = create_method_args("get");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "TEXT(\"/\"), EHttpMethod::Get");
    }

    #[test]
    fn test_http_request_params_empty_path() {
        let path = json!("");
        let args = create_method_args("get");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "TEXT(\"\"), EHttpMethod::Get");
    }

    #[test]
    fn test_convert_to_http_method_all_methods() {
        assert_eq!(convert_to_http_method("get").unwrap(), "Get");
        assert_eq!(convert_to_http_method("post").unwrap(), "Post");
        assert_eq!(convert_to_http_method("put").unwrap(), "Put");
        assert_eq!(convert_to_http_method("delete").unwrap(), "Delete");
        assert_eq!(convert_to_http_method("patch").unwrap(), "Patch");
        assert_eq!(convert_to_http_method("head").unwrap(), "Head");
    }

    #[test]
    fn test_convert_to_http_method_case_insensitive() {
        assert_eq!(convert_to_http_method("GET").unwrap(), "Get");
        assert_eq!(convert_to_http_method("POST").unwrap(), "Post");
        assert_eq!(convert_to_http_method("Put").unwrap(), "Put");
        assert_eq!(convert_to_http_method("DELETE").unwrap(), "Delete");
        assert_eq!(convert_to_http_method("PATCH").unwrap(), "Patch");
        assert_eq!(convert_to_http_method("HEAD").unwrap(), "Head");
    }

    #[test]
    fn test_escape_cpp_string() {
        assert_eq!(escape_cpp_string("simple"), "simple");
        assert_eq!(escape_cpp_string("with\"quote"), "with\\\"quote");
        assert_eq!(escape_cpp_string("with\\backslash"), "with\\\\backslash");
        assert_eq!(escape_cpp_string("both\"and\\here"), "both\\\"and\\\\here");
    }

    #[test]
    fn test_http_request_params_with_special_characters() {
        let path = json!("/api/path\"with\"quotes");
        let args = create_method_args("get");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/api/path\\\"with\\\"quotes\"), EHttpMethod::Get"
        );
    }

    #[test]
    fn test_http_request_params_with_backslash() {
        let path = json!("/api/path\\with\\backslash");
        let args = create_method_args("post");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/api/path\\\\with\\\\backslash\"), EHttpMethod::Post"
        );
    }

    #[test]
    fn test_extract_path_parameters() {
        let params = json!([
            {"in": "path", "name": "id"},
            {"in": "query", "name": "shard"}
        ]);
        assert_eq!(
            extract_path_parameters(params.as_array()),
            vec!["id".to_string()]
        );

        let params_multi = json!([
            {"in": "path", "name": "user_id"},
            {"in": "path", "name": "post_id"},
            {"in": "query", "name": "limit"}
        ]);
        assert_eq!(
            extract_path_parameters(params_multi.as_array()),
            vec!["user_id".to_string(), "post_id".to_string()]
        );

        assert!(extract_path_parameters(None).is_empty());

        let empty_params = json!([]);
        assert!(extract_path_parameters(empty_params.as_array()).is_empty());
    }

    #[test]
    fn test_extract_query_parameters() {
        let params = json!([
            {"in": "query", "name": "shard"},
            {"in": "path", "name": "id"},
            {"in": "query", "name": "limit"}
        ]);

        let result = extract_query_parameters(params.as_array());
        assert_eq!(result, vec!["shard".to_string(), "limit".to_string()]);
    }

    #[test]
    fn test_extract_query_parameters_empty() {
        let result = extract_query_parameters(None);
        assert!(result.is_empty());

        let params = json!([]);
        let result = extract_query_parameters(params.as_array());
        assert!(result.is_empty());
    }

    #[test]
    fn test_http_request_params_with_query_parameters() {
        let path = json!("/v1/player/characters");
        let params = json!([
            {
                "in": "query",
                "name": "shard",
                "schema": {"type": "string", "default": "CN-1"}
            }
        ]);
        let args = create_args_with_params("get", Some(params));

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "FString::Format(TEXT(\"/v1/player/characters?shard={shard}\"), FStringFormatNamedArguments{{\"shard\", shard}}), EHttpMethod::Get"
        );
    }

    #[test]
    fn test_http_request_params_with_path_and_query_parameters() {
        let path = json!("/v1/player/characters/{id}");
        let params = json!([
            {"in": "path", "name": "id"},
            {"in": "query", "name": "shard"}
        ]);
        let args = create_args_with_params("get", Some(params));

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "FString::Format(TEXT(\"/v1/player/characters/{id}?shard={shard}\"), FStringFormatNamedArguments{{\"id\", id}, {\"shard\", shard}}), EHttpMethod::Get"
        );
    }

    #[test]
    fn test_http_request_params_with_multiple_query_parameters() {
        let path = json!("/v1/player/characters");
        let params = json!([
            {"in": "query", "name": "shard"},
            {"in": "query", "name": "limit"},
            {"in": "query", "name": "offset"}
        ]);
        let args = create_args_with_params("get", Some(params));

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "FString::Format(TEXT(\"/v1/player/characters?shard={shard}&limit={limit}&offset={offset}\"), FStringFormatNamedArguments{{\"shard\", shard}, {\"limit\", limit}, {\"offset\", offset}}), EHttpMethod::Get"
        );
    }
}
