use std::{fs::File, io::Write, path::PathBuf};

use anyhow::Context;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::{
    diff::{Change, Compatibility},
    hist::classify::ClassifiedChanges,
};

#[derive(Serialize)]
pub struct JsonHistSummaryReport {
    pub changes: Vec<DiffSummary>,
}

#[derive(Serialize)]
pub struct DiffSummary {
    pub version_old: String,
    pub version_new: String,

    pub input_old: PathBuf,
    pub input_new: PathBuf,

    pub breaking_changes: Vec<String>,
    pub dubious_changes: Vec<String>,
    pub compatible_changes: Vec<String>,
}

pub struct JsonHistPrinter {
    output_dir: PathBuf,
}

impl JsonHistPrinter {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    pub fn print(&mut self, changes: &ClassifiedChanges) -> anyhow::Result<()> {
        let mut summary = File::create(self.output_dir.join("report-summary.json"))?;
        print_summary(&mut summary, &changes)?;
        Ok(())
    }
}

fn print_summary(writer: &mut impl Write, changes: &ClassifiedChanges) -> anyhow::Result<()> {
    let mut report = JsonHistSummaryReport {
        changes: Vec::with_capacity(changes.changed_by_version.len()),
    };

    let paths = &changes.inputs;

    for (version, changes) in changes.changed_by_version.iter() {
        let mut changes_by_compat: FxHashMap<Compatibility, Vec<String>> = changes
            .iter()
            .into_group_map_by(|(_name, diff)| diff.semantic.compat())
            .into_iter()
            .map(|(compat, changes)| {
                (
                    compat,
                    changes
                        .into_iter()
                        .map(|(name, _diff)| name.to_owned())
                        .collect_vec(),
                )
            })
            .collect();

        let summary = DiffSummary {
            version_old: version.old.to_string(),
            version_new: version.new.to_string(),
            input_old: paths.get(&version.old).unwrap().to_owned(),
            input_new: paths.get(&version.new).unwrap().to_owned(),
            breaking_changes: changes_by_compat
                .remove(&Compatibility::Breaking)
                .unwrap_or_default(),
            dubious_changes: changes_by_compat
                .remove(&Compatibility::Dubious)
                .unwrap_or_default(),
            compatible_changes: changes_by_compat
                .remove(&Compatibility::BackwardCompatible)
                .unwrap_or_default(),
        };
        report.changes.push(summary);
    }

    serde_json::to_writer_pretty(writer, &report).context("json serialisation failed")?;
    Ok(())
}
