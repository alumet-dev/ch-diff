use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use clang::Clang;
use clap::Parser;
use regex::Regex;

use ch_diff::{
    diff::filter::DiffFilter,
    generate::CodeVersion,
    hist::{
        classify::classify_changes_in_history, codegen::HistCodegen, report::MarkdownHistPrinter,
        version::Version,
    },
};

fn main() -> anyhow::Result<()> {
    // parse args
    let args = Args::parse();

    // init logger
    env_logger::init();

    // init clang library
    let clang = Clang::new().expect("clang initialisation failed");

    // configure filter
    let filter = match args.whitelist {
        Some(path) => DiffFilter::parse_whitelist_file(path).expect("filter initialisation failed"),
        None => DiffFilter::allow_everything(),
    };

    let version_regex = Regex::new(&args.version_regex).context("failed to compile regex")?;

    // compare every file
    let mut files = std::fs::read_dir(&args.input)
        .context("failed to list the content of the input directory")?
        .map(|f| {
            let path = f.unwrap().path();
            let filename = path.file_name().unwrap().to_str().unwrap();
            let groups = version_regex.captures(&filename).unwrap();
            let version = groups.get(1).unwrap().as_str().to_owned();
            let version = Version::from_str(&version).unwrap();
            // TODO also get the lib version, from the #define of the header
            (path, version)
        })
        .collect::<Vec<_>>();

    files.sort_by_key(|(_, version)| version.clone());
    let changes = classify_changes_in_history(&files, &clang, &filter);

    // emit reports
    let mut printer = MarkdownHistPrinter::new(args.output_dir.clone());
    printer.print(&changes).context("failed to emit reports")?;

    // print code
    if let Some(code_version) = args.generate_code {
        let mut generator = HistCodegen::new(args.output_dir, code_version);
        generator
            .generate_partial_version(changes)
            .context("failed to generate partial versions")?;
    }

    Ok(())
}

#[derive(clap::Parser)]
#[command(version, about)]
struct Args {
    /// Input directory that contains the successive versions of the header.
    #[arg(long, short)]
    input: PathBuf,

    /// Regular expression that extracts the version number from the header file name.
    #[arg(default_value = ".*-([\\d.a-z]+)\\.h")]
    version_regex: String,

    /// Output directory.
    #[arg(short, long)]
    output_dir: PathBuf,

    /// Save the changes between each version in .h files.
    #[arg(long)]
    generate_code: Option<CodeVersion>,

    /// Path to the whitelist file.
    ///
    /// Only the names (one per line) contained in this file will show up in the report.
    #[arg(long)]
    whitelist: Option<PathBuf>,
}
