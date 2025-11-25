/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

use std::collections::HashMap;
use tera::{to_value, Result, Value};

/// Tera filter to transform an array of tags into a pipe-separated string.
///
/// This filter takes an array of strings (tags) and joins them with a pipe (`|`) delimiter.
/// For example, `["Character", "Inventory"]` becomes `"Character|Inventory"`.
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

    // 3. Join with a pipe delimiter
    let result = tag_strings.join("|");

    // 4. Return as Value
    to_value(result)
        .map_err(|e| tera::Error::msg(format!("Failed to convert string to Value: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        // Test with an array containing non-string elements
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
