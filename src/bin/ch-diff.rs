use std::path::PathBuf;

use anyhow::Context;
use clang::*;
use clap::Parser;

use ch_diff::{
    ast::Header,
    diff::{filter::DiffFilter, report::DiffReport},
    generate::{CodeGenerator, CodeVersion, CodegenOptions},
    print::{AnsiPrinter, ReportPrinter, ansi::AnsiOptions, types::TypePrintingStyle},
};

fn main() {
    // parse args
    let args = Args::parse();

    // init logger
    env_logger::init();

    // init clang library
    let clang = Clang::new().expect("clang initialisation failed");

    // parse both versions
    let old_header = Header::parse(&clang, &args.old_file).unwrap();
    let new_header = Header::parse(&clang, &args.new_file).unwrap();

    // configure filter
    let filter = match args.whitelist {
        Some(path) => DiffFilter::parse_whitelist_file(path).expect("filter initialisation failed"),
        None => DiffFilter::allow_everything(),
    };

    // compute diff
    let report = DiffReport::compute_diff(&old_header, &new_header, &filter)
        .with_context(|| {
            format!(
                "failed to compute diff between {:?} and {:?}",
                args.old_file, args.new_file
            )
        })
        .unwrap();

    // print diff, the only output format that we support for now is colored ansi text
    let options = AnsiOptions {
        type_style: args.types,
        print_diff_sign: false,
    };
    match args.report_output {
        Some(f) => {
            let mut printer = AnsiPrinter::create_file(f.as_path(), options).unwrap();
            printer.print_report(&report).expect("printing error");
        }
        None => {
            let mut printer = AnsiPrinter::to_stdout(options).unwrap();
            printer.print_report(&report).expect("printing error");
        }
    }

    // print code
    let options = CodegenOptions {
        version: args.codegen_version.unwrap_or(CodeVersion::Old),
    };
    if let Some(f) = args.codegen_output {
        let mut generator = CodeGenerator::create_file(f.as_path(), options).unwrap();
        generator.generate_code(&report).expect("codegen error");
    }
}

#[derive(clap::Parser)]
#[command(version, about)]
struct Args {
    /// Old version of the C header.
    old_file: PathBuf,
    /// New version of the C header.
    new_file: PathBuf,

    /// Report output file. By default, we print to stdout.
    #[arg(long, short)]
    report_output: Option<PathBuf>,

    /// Version to output if codegen is enabled.
    #[arg(long)]
    codegen_version: Option<CodeVersion>,

    /// Code output file. By default, codegen is disabled.
    #[arg(long)]
    codegen_output: Option<PathBuf>,

    /// How to print types in the report.
    #[arg(long, default_value = "rust")]
    types: TypePrintingStyle,

    /// Path to the whitelist file.
    ///
    /// Only the names (one per line) contained in this file will show up in the report.
    #[arg(long)]
    whitelist: Option<PathBuf>,
}
