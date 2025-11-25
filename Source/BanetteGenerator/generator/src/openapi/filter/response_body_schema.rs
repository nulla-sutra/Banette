use std::collections::HashMap;
use tera::{Result, Value};

/// Successful HTTP status codes to prioritize when extracting response schemas
const SUCCESS_STATUS_CODES: &[&str] = &["200", "201", "202", "203", "204"];

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
    use tera::to_value;

    use crate::openapi::filter::to_ue_type::to_ue_type_filter;

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
}
