use anyhow::{Context, Result};
use oas3::{Spec, from_json, from_yaml};
use std::fs;

/// Format of the OpenAPI specification file.
#[derive(Debug)]
pub enum Format {
    Json,
    Yaml,
}

/// Infers the format from the path/URL suffix.
fn infer_format(path: &str) -> Result<Format> {
    if path.ends_with(".json") {
        Ok(Format::Json)
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        Ok(Format::Yaml)
    } else {
        anyhow::bail!(
            "Failed to detect OpenAPI format from path: {}. Expected .json, .yaml, or .yml suffix",
            path
        )
    }
}

pub fn load_openapi_spec(path: &str) -> Result<Spec> {
    let format = infer_format(path).context("Failed to detect OpenAPI format from path")?;

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

    match format {
        Format::Json => {
            let spec_json: serde_json::Value =
                serde_json::from_str(&raw_spec).context("Failed to parse initial JSON content")?;

            // Re-serialize to pretty string for debugging purposes
            let pretty_str = serde_json::to_string_pretty(&spec_json)
                .context("Failed to normalize JSON structure")?;

            from_json(&pretty_str).context("Failed to parse into OpenAPI Spec object")
        }
        Format::Yaml => {
            // Validate YAML with serde_yaml_bw before parsing with oas3
            let _: serde_yaml_bw::Value = serde_yaml_bw::from_str(&raw_spec)
                .context("Failed to parse initial YAML content with serde-yaml-bw")?;

            from_yaml(&raw_spec).context("Failed to parse YAML into OpenAPI Spec object")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    //noinspection SpellCheckingInspection
    #[test]
    #[ignore = "Requires a JSON endpoint to be running"]
    fn test_load_openapi_spec() {
        load_openapi_spec("http://127.0.0.1:10802/docs/api.json").unwrap();
    }

    #[test]
    #[ignore = "Requires a YAML endpoint to be running"]
    fn test_load_openapi_spec_yaml() {
        load_openapi_spec("http://127.0.0.1:10802/docs/api.yaml").unwrap();
    }

    #[test]
    fn test_load_openapi_spec_local_yaml() {
        let yaml_content = r#"
openapi: "3.1.0"
info:
  title: Test API
  version: "1.0.0"
paths: {}
"#;
        // Write to a temp file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_openapi.yaml");
        let mut file = fs::File::create(&temp_file).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let result = load_openapi_spec(temp_file.to_str().unwrap());
        assert!(
            result.is_ok(),
            "Failed to load YAML spec: {:?}",
            result.err()
        );

        let spec = result.unwrap();
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.info.version, "1.0.0");

        // Cleanup
        fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_load_openapi_spec_local_yml() {
        let yaml_content = r#"
openapi: "3.1.0"
info:
  title: YML Extension Test
  version: "2.0.0"
paths: {}
"#;
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_openapi.yml");
        let mut file = fs::File::create(&temp_file).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let result = load_openapi_spec(temp_file.to_str().unwrap());
        assert!(
            result.is_ok(),
            "Failed to load YML spec: {:?}",
            result.err()
        );

        let spec = result.unwrap();
        assert_eq!(spec.info.title, "YML Extension Test");

        fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_load_openapi_spec_local_json() {
        let json_content = r#"{
  "openapi": "3.1.0",
  "info": {
    "title": "JSON Test API",
    "version": "3.0.0"
  },
  "paths": {}
}"#;
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_openapi.json");
        let mut file = fs::File::create(&temp_file).unwrap();
        file.write_all(json_content.as_bytes()).unwrap();

        let result = load_openapi_spec(temp_file.to_str().unwrap());
        assert!(
            result.is_ok(),
            "Failed to load JSON spec: {:?}",
            result.err()
        );

        let spec = result.unwrap();
        assert_eq!(spec.info.title, "JSON Test API");
        assert_eq!(spec.info.version, "3.0.0");

        fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_infer_format_json() {
        assert!(matches!(
            infer_format("path/to/spec.json").unwrap(),
            Format::Json
        ));
    }

    #[test]
    fn test_infer_format_yaml() {
        assert!(matches!(
            infer_format("path/to/spec.yaml").unwrap(),
            Format::Yaml
        ));
    }

    #[test]
    fn test_infer_format_yml() {
        assert!(matches!(
            infer_format("path/to/spec.yml").unwrap(),
            Format::Yaml
        ));
    }

    #[test]
    fn test_infer_format_http_json() {
        assert!(matches!(
            infer_format("https://example.com/openapi.json").unwrap(),
            Format::Json
        ));
    }

    #[test]
    fn test_infer_format_http_yaml() {
        assert!(matches!(
            infer_format("http://example.com/spec.yaml").unwrap(),
            Format::Yaml
        ));
    }

    #[test]
    fn test_infer_format_unknown() {
        let result = infer_format("path/to/spec.txt");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to detect OpenAPI format"));
    }

    #[test]
    fn test_infer_format_no_extension() {
        let result = infer_format("path/to/spec");
        assert!(result.is_err());
    }
}
