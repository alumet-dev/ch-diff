use std::path::PathBuf;

use anyhow::Context;
use clang::*;
use clap::Parser;

use ch_diff::{
    ast::HeaderContent,
    diff::DiffReport,
    print::{AnsiPrinter, ReportPrinter},
};

fn main() {
    // parse args
    let args = Args::parse();

    // init logger
    env_logger::init();

    // init clang library
    let clang = Clang::new().expect("clang initialisation failed");

    // parse both versions
    let old_header = parse(&clang, &args.old_file).unwrap();
    let new_header = parse(&clang, &args.new_file).unwrap();
    let old_name = args.old_file.to_str().unwrap();
    let new_name = args.new_file.to_str().unwrap();

    // compute diff
    let report = DiffReport::compute_diff((old_name, &old_header), (new_name, &new_header))
        .with_context(|| {
            format!(
                "failed to compute diff between {:?} and {:?}",
                args.old_file, args.new_file
            )
        })
        .unwrap();

    // print diff, the only output format that we support for now is colored ansi text
    match args.output {
        Some(f) => {
            let mut printer = AnsiPrinter::create_file(f.as_path()).unwrap();
            printer.print_report(report).expect("printing error");
        }
        None => {
            let mut printer = AnsiPrinter::to_stdout();
            printer.print_report(report).expect("printing error");
        }
    }
}

fn parse<'a>(clang: &'a Clang, file: &PathBuf) -> anyhow::Result<HeaderContent> {
    let index = Index::new(&clang, true, true);
    let tu = index
        .parser(&file)
        .parse()
        .with_context(|| format!("failed to parse {file:?}"))?;
    HeaderContent::analyse(tu).with_context(|| format!("failed to analyse {file:?}"))
}

#[derive(clap::Parser)]
#[command(version, about)]
struct Args {
    /// Old version of the C header.
    old_file: PathBuf,
    /// New version of the C header.
    new_file: PathBuf,

    /// Output file. By default, we print to stdout.
    #[arg(long, short)]
    output: Option<PathBuf>,
}
