use std::collections::HashMap;
use tera::{Result, Value};

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
