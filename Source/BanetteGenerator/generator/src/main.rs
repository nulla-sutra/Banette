/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Mode {
    Openapi,
    UStruct,
}
#[derive(Parser)]
struct Args {
    #[arg(short, long, value_enum, default_value_t = Mode::Openapi)]
    mode: Mode,
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

    match args.mode {
        Mode::Openapi => generator::openapi::generate_safe(
            args.path.as_str(),
            args.output_dir.as_str(),
            args.file_name.as_str(),
            args.module_name.as_str(),
            generator::openapi::parser::parse_include_headers(&args.extra_headers),
        ),
        Mode::UStruct => {
            unimplemented!();
        }
    }
}
