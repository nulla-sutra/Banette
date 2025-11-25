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
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    generate_safe(
        args.path.as_str(),
        args.output_dir.as_str(),
        args.file_name.as_str(),
        args.module_name.as_str(),
    )
}
