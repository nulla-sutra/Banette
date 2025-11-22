use oas3::{Spec, from_json};
use std::ffi::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn generate(openapi_path: *const c_char, output_dir: *const c_char) {}


fn load_openapi_spec(path: &str) -> Result<Spec, String> {
    // url
    let spec_str = if path.starts_with("http://") || path.starts_with("https://") {
        ureq::get(path)
            .call()
            .map_err(|e| format!("Cannot fetch OpenAPI spec: {e}"))?
            .into_body()
            .read_to_string()
            .map_err(|e| format!("Cannot read HTTP body: {e}"))?
    }
    // local file
    else {
        std::fs::read_to_string(path).map_err(|e| format!("Cannot read file `{path}`: {e}"))?
    };

    let v: serde_json::Value = serde_json::from_str(&spec_str)
        .map_err(|e| format!("Cannot parse OpenAPI spec `{path}`: {e}"))?;

    let pretty_str = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;

    // println!("{}", pretty_str);

    // Cannot compile currently due to https://github.com/x52dev/oas3-rs/issues/278
    from_json(&pretty_str).map_err(|e| format!("Cannot parse OpenAPI spec `{path}`: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_openapi_spec() -> Result<(), String> {
        let spec = load_openapi_spec("http://127.0.0.1:10802/docs/api.json")?;
        assert!(spec.info.title.len() > 0);
        Ok(())
    }
}
