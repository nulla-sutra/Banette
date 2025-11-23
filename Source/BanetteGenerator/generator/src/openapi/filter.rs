use std::collections::HashMap;
use tera::{Result, Value, to_value};

pub(crate) fn to_ue_type_filter(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    fn get_cpp_type(schema: &Value) -> String {
        // 1. 处理布尔值 Schema (true/false)
        if let Some(is_any) = schema.as_bool() {
            return if is_any {
                "FInstancedStruct".to_string() // Any type
            } else {
                "void*".to_string() // Impossible type
            };
        }

        // 2. 处理 $ref 引用
        // 如果存在 $ref，直接返回对应的结构体名称，不需要继续递归
        if let Some(ref_path) = schema.get("$ref").and_then(|v| v.as_str()) {
            let struct_name = ref_path.split('/').last().unwrap_or("Unknown");
            return format!("F{}", struct_name);
        }

        // 3. 获取类型字符串
        let type_str = schema
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("object");

        match type_str {
            "string" => "FString".to_string(),
            "integer" => {
                // 可选：检查 format 区分 int32/int64
                let format = schema.get("format").and_then(|f| f.as_str());
                match format {
                    Some("int64") => "int64".to_string(),
                    _ => "int32".to_string(),
                }
            }
            "number" => "float".to_string(),
            "boolean" => "bool".to_string(),
            "array" => {
                // === 递归重点 ===
                // 获取 items 字段
                if let Some(items) = schema.get("items") {
                    // 递归调用自己来获取内部类型
                    let inner_type = get_cpp_type(items);
                    format!("TArray<{}>", inner_type)
                } else {
                    // 如果是数组但没有 items 定义，假定为任意类型数组
                    "TArray<FInstancedStruct>".to_string()
                }
            }
            // object 或其他情况
            _ => "FInstancedStruct".to_string(),
        }
    }

    let result = get_cpp_type(value);
    Ok(to_value(result)?)
}

/// Tera filter to check if a property is required.
///
/// Usage in the template: {{ prop_name | is_required(required_list=schema.required) }}
pub(crate) fn is_required_filter(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    // 1. 获取要检查的属性名称 (prop_name)
    let prop_name = value.as_str().ok_or_else(|| {
        tera::Error::msg("is_required filter expects property name as input string.")
    })?;

    // 2. 获取作为参数传入的 'required_list' 数组
    let required_list = args
        .get("required_list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            tera::Error::msg(
                "is_required filter requires 'required_list' argument, and it must be an array.",
            )
        })?;

    // 3. 在数组中查找属性名称
    let is_required = required_list.iter().any(|v| v.as_str() == Some(prop_name));

    // 4. 返回布尔值
    tera::to_value(is_required)
        .map_err(|e| tera::Error::msg(format!("Failed to convert bool to Value: {}", e)))
}

/// 将 OpenAPI 路径转换为 PascalCase 函数名，并追加 HTTP 方法。
///
/// 示例: /v1/player/characters, method="get" -> V1PlayerCharacters_GET
pub(crate) fn path_to_func_name_filter(
    value: &Value,
    args: &HashMap<String, Value>,
) -> Result<Value> {
    let path = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("Path must be a string"))?;

    // 1. 获取并大写 HTTP 方法 (GET, POST, etc.)
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("path_to_func_name requires a 'method' argument"))?
        .to_uppercase();

    // 2. 移除开头的斜杠
    let cleaned_path = path.trim_start_matches('/');

    // 3. 分割并应用 PascalCase 转换
    let parts: Vec<&str> = cleaned_path.split('/').collect();
    let mut func_base_name = String::new();

    for part in parts {
        if part.is_empty() {
            continue;
        }

        let mut chars = part.chars();
        // Capitalize the first character
        if let Some(first_char) = chars.next() {
            func_base_name.push_str(&first_char.to_uppercase().to_string());
            // Append the rest
            func_base_name.push_str(chars.as_str());
        }
    }

    // 4. 合并函数名和方法
    let final_name = format!("{}_{}", func_base_name, method);

    Ok(to_value(final_name)?)
}

pub fn get_body_schema_filter(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    // 1. 检查输入是否为对象
    let req_body = value.as_object().ok_or_else(|| {
        tera::Error::msg("Input to get_body_schema must be a valid requestBody object.")
    })?;

    // 2. 获取 "content" 字段
    let content = req_body
        .get("content")
        .ok_or_else(|| tera::Error::msg("requestBody object is missing 'content' field."))?;

    // 3. 尝试找到 "application/json" 对应的 schema
    if let Some(schema_obj) = content
        .get("application/json")
        .and_then(|json_media_type| json_media_type.get("schema"))
    {
        return Ok(schema_obj.clone());
    }

    // 4. 回退机制：如果没有 application/json，则尝试第一个可用的媒体类型
    if let Some(content_map) = content.as_object() {
        if let Some((_, media_type)) = content_map.iter().next() {
            if let Some(schema_obj) = media_type.get("schema") {
                return Ok(schema_obj.clone());
            }
        }
    }

    // 5. 失败处理
    Err(tera::Error::msg(
        "Could not find a valid schema object within requestBody content (checked application/json and first available type).",
    ))
}
