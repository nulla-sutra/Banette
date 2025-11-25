use std::collections::HashMap;
use tera::{to_value, Result, Value};

/// Tera filter to assemble FHttpRequest constructor parameters from a path-item.
///
/// This filter takes a path string and HTTP method, then generates the parameters
/// needed to construct an FHttpRequest in Unreal Engine C++.
///
/// FHttpRequest has the following fields:
/// - Url: FString - The absolute URL to call
/// - Method: EHttpMethod - The HTTP verb (Get, Post, Put, Delete, Patch, Head)
///
/// Usage in the template: {{ path | http_request_params(method=method) }}
///
/// Examples:
/// - `/v1/player/characters`, method="get" -> `TEXT("/v1/player/characters"), EHttpMethod::Get`
/// - `/character/{id}`, method="post" -> `TEXT("/character/{id}"), EHttpMethod::Post`
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

    // 3. Convert the HTTP method to EHttpMethod enum value
    let http_method = convert_to_http_method(method)?;

    // 4. Escape special characters in the path for C++ string literal
    let escaped_path = escape_cpp_string(path);

    // 5. Build the constructor parameters string
    // Format: TEXT("path"), EHttpMethod::Method
    let params = format!("TEXT(\"{}\"), EHttpMethod::{}", escaped_path, http_method);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::filter::tests::create_method_args;
    use serde_json::json;

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
        let args = create_method_args("post");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/character/{id}\"), EHttpMethod::Post"
        );
    }

    #[test]
    fn test_http_request_params_put_method() {
        let path = json!("/api/resource/{id}");
        let args = create_method_args("put");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/api/resource/{id}\"), EHttpMethod::Put"
        );
    }

    #[test]
    fn test_http_request_params_delete_method() {
        let path = json!("/items/{item_id}");
        let args = create_method_args("delete");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/items/{item_id}\"), EHttpMethod::Delete"
        );
    }

    #[test]
    fn test_http_request_params_patch_method() {
        let path = json!("/user/{user_id}/profile");
        let args = create_method_args("patch");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/user/{user_id}/profile\"), EHttpMethod::Patch"
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
        let args = create_method_args("get");

        let result = http_request_params_filter(&path, &args).unwrap();
        assert_eq!(
            result.as_str().unwrap(),
            "TEXT(\"/api/v2/{resource_id}/sub/{sub_id}/details\"), EHttpMethod::Get"
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
}
