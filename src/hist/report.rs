use std::{fs::File, io::Write, path::PathBuf};

use itertools::Itertools;

use crate::{
    diff::{Change, Compatibility},
    hist::classify::ClassifiedChanges,
};

pub struct MarkdownHistPrinter {
    output_dir: PathBuf,
}

impl MarkdownHistPrinter {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    pub fn print(&mut self, changes: ClassifiedChanges) -> anyhow::Result<()> {
        let mut summary = File::create(self.output_dir.join("report-summary.md"))?;
        let mut details = File::create(self.output_dir.join("report-details.md"))?;
        print_summary(&mut summary, &changes)?;
        print_details(&mut details, &changes)?;
        Ok(())
    }
}

fn compat_to_ascii(compat: Compatibility) -> &'static str {
    match compat {
        Compatibility::Breaking => "(!)",
        Compatibility::Dubious => "(?)",
        Compatibility::BackwardCompatible => "(-)",
    }
}

fn print_summary(writer: &mut impl Write, changes: &ClassifiedChanges) -> anyhow::Result<()> {
    writeln!(writer, "# History Report (summary)")?;
    writeln!(writer)?;

    // stable
    writeln!(writer, "## Stable Symbols")?;
    writeln!(writer)?;
    for name in changes.stable.iter().sorted() {
        writeln!(writer, "- {name}")?;
    }
    writeln!(writer)?;

    // unstable
    writeln!(writer, "## Unstable Symbols")?;
    writeln!(writer)?;
    for (name, compat) in changes.changed.iter().sorted() {
        writeln!(writer, "- {name} {}", compat_to_ascii(*compat))?;
    }
    writeln!(writer)?;

    // version history
    writeln!(writer, "## Changes History")?;
    writeln!(writer)?;
    for (version, changes) in changes.changed_by_version.iter() {
        writeln!(writer, "### Changed in {version}")?;
        writeln!(writer)?;
        for (name, diff) in changes {
            writeln!(
                writer,
                "- {name} {}",
                compat_to_ascii(diff.semantic.compat())
            )?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn print_details(writer: &mut impl Write, changes: &ClassifiedChanges) -> anyhow::Result<()> {
    writeln!(writer, "# History Report (unstable details)")?;
    writeln!(writer)?;

    for (version, changes) in changes.changed_by_version.iter() {
        writeln!(writer, "### Changed in {version}")?;
        writeln!(writer)?;
        for (name, diff) in changes {
            writeln!(
                writer,
                "#### {name} {}",
                compat_to_ascii(diff.semantic.compat())
            )?;
            writeln!(writer)?;

            writeln!(writer, "```c")?;
            for line in diff.source.old.lines() {
                writeln!(writer, "-{line}")?;
            }
            for line in diff.source.new.lines() {
                writeln!(writer, "+{line}")?;
            }
            writeln!(writer, "```")?;
        }
        writeln!(writer)?;
    }
    Ok(())
}
