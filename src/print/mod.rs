use crate::diff::report::DiffReport;

pub mod ansi;
pub mod types;

pub trait ReportPrinter {
    fn print_report(&mut self, report: &DiffReport) -> anyhow::Result<()>;
}

pub use ansi::AnsiPrinter;
