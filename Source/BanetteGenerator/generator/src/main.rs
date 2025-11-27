/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

use clap::Parser;
use generator::openapi::generate_safe;
use generator::openapi::parser::parse_include_headers;

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

    let include_headers = parse_include_headers(&args.extra_headers);

    generate_safe(
        args.path.as_str(),
        args.output_dir.as_str(),
        args.file_name.as_str(),
        args.module_name.as_str(),
        include_headers,
    )
}
