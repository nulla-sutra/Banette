use std::collections::HashMap;
use tera::{Result, Value, to_value};

pub fn to_ue_type_filter(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    fn get_cpp_type(schema: &Value) -> String {
        // 1. Handle boolean Schema (true/false)
        if let Some(is_any) = schema.as_bool() {
            return if is_any {
                "FInstancedStruct".to_string() // Any type
            } else {
                "void*".to_string() // Impossible type
            };
        }

        // 2. Handle $ref references
        // If $ref exists, return the corresponding struct name directly; no need to recurse further
        if let Some(ref_path) = schema.get("$ref").and_then(|v| v.as_str()) {
            let struct_name = ref_path.split('/').last().unwrap_or("Unknown");
            return format!("F{}", struct_name);
        }

        // 3. Get the type string
        let type_str = schema
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("object");

        match type_str {
            "string" => "FString".to_string(),
            "integer" => {
                // Optional: check 'format' to distinguish int32/int64
                let format = schema.get("format").and_then(|f| f.as_str());
                match format {
                    Some("int64") => "int64".to_string(),
                    _ => "int32".to_string(),
                }
            }
            "number" => "float".to_string(),
            "boolean" => "bool".to_string(),
            "array" => {
                // === Recursion key point ===
                // Get the 'items' field
                if let Some(items) = schema.get("items") {
                    // Recursively call itself to get the inner type
                    let inner_type = get_cpp_type(items);
                    format!("TArray<{}>", inner_type)
                } else {
                    // If it's an array without 'items' defined, assume an array of any type
                    "TArray<FInstancedStruct>".to_string()
                }
            }
            // object or other cases
            _ => "FInstancedStruct".to_string(),
        }
    }

    let result = get_cpp_type(value);
    Ok(to_value(result)?)
}
