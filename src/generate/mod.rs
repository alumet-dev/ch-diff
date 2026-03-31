//! Generate code for multi-version support.

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::Context;

use crate::diff::report::DiffReport;

pub struct CodeGenerator<W: Write> {
    writer: W,
    options: CodegenOptions,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, clap::ValueEnum)]
pub enum CodeVersion {
    Old,
    New,
}

pub struct CodegenOptions {
    pub version: CodeVersion,
}

impl<W: Write> CodeGenerator<W> {
    fn new(writer: W, options: CodegenOptions) -> anyhow::Result<Self> {
        Ok(Self { writer, options })
    }

    pub fn generate_code(&mut self, report: &DiffReport) -> anyhow::Result<()> {
        for diffs in report.declarations.values() {
            for diff in diffs.values() {
                let code = match self.options.version {
                    CodeVersion::Old => &diff.source.old,
                    CodeVersion::New => &diff.source.new,
                };
                writeln!(self.writer, "{code}")?;
            }
        }
        Ok(())
    }
}

impl CodeGenerator<BufWriter<File>> {
    pub fn to_file(output: File, options: CodegenOptions) -> anyhow::Result<Self> {
        Self::new(BufWriter::new(output), options)
    }

    pub fn create_file(output: &Path, options: CodegenOptions) -> anyhow::Result<Self> {
        let output = File::create(output)
            .with_context(|| format!("could not create new file {output:?}"))?;
        Self::to_file(output, options)
    }
}
