mod openapi;

use crate::openapi::filter::{
    is_required_filter, path_to_func_name_filter, request_body_schema_filter,
    response_body_schema_filter, tags_to_pipe_separated_filter, to_ue_type_filter,
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

/// Generates a safely rendered output file based on an OpenAPI specification and
/// template, with the ability to customize the target filename and module name.
///
/// # Parameters
/// - `path`: A string slice representing the file system path to the OpenAPI specification file.
/// - `output_dir`: A string slice specifying the directory where the generated file should be saved.
/// - `file_name`: The desired name for the generated file.
/// - `module_name`: The module name to be used in the rendered output.
///
/// # Returns
/// - `anyhow::Result<()>`: Returns `Ok(())` if the operation completes successfully, or an error
///   wrapped in `anyhow::Result` if any step of the generation process fails.
///
/// # Behavior
/// 1. Loads the OpenAPI specification from the file located at the provided `path`.
/// 2. Initializes a Tera template engine instance for rendering templates.
/// 3. Ensures the existence of the `output_dir`, creating the directory if it is missing.
/// 4. Registers custom Tera filters that provide specific processing utilities during rendering:
///    - `to_ue_type`: Converts to an Unreal Engine type.
///    - `is_required`: Determines if a field is required.
///    - `path_to_func_name`: Converts a path to a function-friendly name.
///    - `request_body_schema`: Extracts the request body schema.
///    - `response_body_schema`: Extracts the response body schema.
///    - `tags_to_pipe_separated`: Converts tags into a pipe-separated format.
/// 5. Loads the OpenAPI template:
///    - In debug mode, it reads the template file from the filesystem.
///    - In release mode, it embeds the template as a raw string during compilation.
/// 6. Creates a rendering context using the deserialized data from the OpenAPI spec and additional inputs:
///    - Inserts `module_name` and `file_name` into the context for further customization in the template.
/// 7. Uses the Tera engine to render the template into a file format.
///
/// # Side Effects
/// - Writes a generated file to the specified `output_dir` under the provided `file_name`.
///
/// # Errors
/// - Returns an error if:
///   - The OpenAPI specification cannot be loaded.
///   - The `output_dir` cannot be created or accessed.
///   - The template file cannot be read or added.
///   - The rendering process fails due to invalid data or template.
///   - The output file cannot be written to disk.
///
/// # Example
/// ```rust
/// use anyhow::Result;
///
/// fn main() -> Result<()> {
///     generate_safe(
///         "path/to/openapi.yaml",
///         "output/directory",
///         "generated_file.h",
///         "MyModule",
///     )?;
///     Ok(())
/// }
/// ```
///
/// In this example:
/// - The function reads the OpenAPI spec from `path/to/openapi.yaml`.
/// - Generates a file named `generated_file.h` in the `output/directory`.
/// - Writes the rendered file using the specified `MyModule` name in the template.
pub fn generate_safe(
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
    tera.register_filter("request_body_schema", request_body_schema_filter);
    tera.register_filter("response_body_schema", response_body_schema_filter);
    tera.register_filter("tags_to_pipe_separated", tags_to_pipe_separated_filter);

    #[cfg(debug_assertions)]
    {
        let template_path = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/api.h.tera");
        tera.add_template_file(template_path, Some("open_api_template"))?;
    }

    #[cfg(not(debug_assertions))]
    {
        tera.add_raw_template(
            "open_api_template",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/api.h.tera")),
        )?;
    }

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
