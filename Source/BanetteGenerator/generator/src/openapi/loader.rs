use anyhow::{Context, Result};
use oas3::{Spec, from_json};
use std::fs;

pub(crate) fn load_openapi_spec(path: &str) -> Result<Spec> {
    let raw_spec = if path.starts_with("http://") || path.starts_with("https://") {
        ureq::get(path)
            .call()
            .context("Failed to make HTTP request")?
            .into_body()
            .read_to_string()
            .context("Failed to read HTTP response body")?
    } else {
        fs::read_to_string(path)
            .with_context(|| format!("Failed to read local file at: {}", path))?
    };

    let spec_json: serde_json::Value =
        serde_json::from_str(&raw_spec).context("Failed to parse initial JSON content")?;

    // Re-serialize to pretty string for debugging purposes

    let pretty_str =
        serde_json::to_string_pretty(&spec_json).context("Failed to normalize JSON structure")?;

    from_json(&pretty_str).context("Failed to parse into OpenAPI Spec object")
}

#[cfg(test)]
mod tests {
    use super::*;

    //noinspection SpellCheckingInspection
    #[test]
    fn test_load_openapi_spec() {
        load_openapi_spec("http://127.0.0.1:10802/docs/api.json").unwrap();
    }
}
