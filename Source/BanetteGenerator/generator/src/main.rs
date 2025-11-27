/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

use clap::Parser;
use generator::openapi::generate_safe;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    path: String,
    #[arg(long)]
    output_dir: String,
    #[arg(long)]
    file_name: String,
    #[arg(long)]
    module_name: String,
    #[arg(long, default_value = "")]
    extra_headers: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Parse extra_headers if provided
    let include_headers: Vec<String> = if args.extra_headers.is_empty() {
        Vec::new()
    } else {
        args.extra_headers
            .split("#include")
            .filter_map(|part| {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    let mut header = format!("#include {}", trimmed);
                    if !header.ends_with(';') {
                        header.push(';');
                    }
                    Some(header)
                }
            })
            .collect()
    };

    generate_safe(
        args.path.as_str(),
        args.output_dir.as_str(),
        args.file_name.as_str(),
        args.module_name.as_str(),
        include_headers,
    )
}
