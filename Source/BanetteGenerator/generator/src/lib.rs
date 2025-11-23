mod openapi;

use crate::openapi::filter::{
    get_request_body_schema_filter, is_required_filter, path_to_func_name_filter, to_ue_type_filter,
};
use crate::openapi::loader::load_openapi_spec;
use anyhow::anyhow;
use std::ffi::{CStr, c_char};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tera::Tera;

#[unsafe(no_mangle)]
pub extern "C" fn generate(
    openapi_path: *const c_char,
    output_dir: *const c_char,
    file_name: *const c_char,
    module_name: *const c_char,
) {
    let result = (|| -> anyhow::Result<()> {
        let convert_arg = |ptr: *const c_char, param_name: &str| -> anyhow::Result<&str> {
            if ptr.is_null() {
                anyhow::bail!("Argument cannot be null (received NULL pointer)",);
            }
            // SAFETY: CStr::from_ptr is safe because we check for null.
            unsafe { CStr::from_ptr(ptr) }
                .to_str()
                .map_err(|e| anyhow!("Argument {} contains invalid UTF-8: {}", param_name, e))
        };

        let openapi_path = convert_arg(openapi_path, "openapi_path")?;
        let output_dir = convert_arg(output_dir, "output_dir")?;
        let file_name = convert_arg(file_name, "file_name")?;
        let module_name = convert_arg(module_name, "module_name")?;

        generate_safe(openapi_path, output_dir, file_name, module_name)
    })();

    if let Err(e) = result {
        eprintln!("[Rust] Generation failed: {}", e);
    } else {
        println!("[Rust] Code generation completed successfully.");
    }
}

fn generate_safe(
    path: &str,
    output_dir: &str,
    file_name: &str,
    module_name: &str,
) -> anyhow::Result<()> {
    let spec = load_openapi_spec(path)?;
    let mut tera = Tera::default();

    let out_path = Path::new(output_dir);

    if !out_path.exists() {
        fs::create_dir_all(out_path)?;
    }

    let file_path = out_path.join(file_name);

    let file_name_base = file_path.file_stem().unwrap_or_default().to_string_lossy();

    tera.register_filter("to_ue_type", to_ue_type_filter);
    tera.register_filter("is_required", is_required_filter);
    tera.register_filter("path_to_func_name", path_to_func_name_filter);
    tera.register_filter("get_request_body_schema", get_request_body_schema_filter);

    tera.add_template_file("templates/api.h.tera", Some("open_api_template"))?;

    let mut context = tera::Context::from_serialize(&spec)?;
    context.insert("module_name", &module_name);
    context.insert("file_name", &file_name_base);

    let rendered = tera.render("open_api_template", &context)?;

    let mut file = File::create(&file_path)?;

    file.write_all(rendered.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    //noinspection ALL
    #[test]
    fn test_generate() {
        generate_safe(
            "http://127.0.0.1:10802/docs/api.json",
            "./target/test",
            "AnxApi.h",
            "ANXNET_API",
        )
        .unwrap();
    }
}
