use std::{collections::BTreeMap, fs::File, io::Write, path::PathBuf};

use anyhow::Context;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::{
    diff::{Change, Compatibility},
    hist::{classify::ClassifiedChanges, version::Version},
};

#[derive(Serialize)]
pub struct JsonHistSummaryReport {
    /// Stable API throughout all analysed versions.
    pub stable: Vec<String>,

    /// Unstable API throughout all analysed versions.
    pub unstable: ChangedSymbols,

    /// Changes between each successive version.
    pub changes_per_version: Vec<HeaderDiffSummary>,

    /// Changes for each symbol.
    pub changes_per_symbol: BTreeMap<String, BTreeMap<Version, Compatibility>>,
}

#[derive(Serialize)]
pub struct ChangedSymbols {
    pub breaking: Vec<String>,
    pub dubious: Vec<String>,
    pub compatible: Vec<String>,
}

#[derive(Serialize)]
pub struct HeaderDiffSummary {
    pub version_old: String,
    pub version_new: String,

    pub input_old: PathBuf,
    pub input_new: PathBuf,

    pub changed: ChangedSymbols,
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
    fn changes_by_compat<'a>(
        changes: impl Iterator<Item = (&'a String, Compatibility)>,
    ) -> FxHashMap<Compatibility, Vec<String>> {
        changes
            .into_group_map_by(|(_name, compat)| *compat)
            .into_iter()
            .map(|(compat, changes)| {
                (
                    compat,
                    changes
                        .into_iter()
                        .map(|(name, _compat)| name.to_owned())
                        .collect_vec(),
                )
            })
            .collect()
    }

    let mut unstable_by_compat =
        changes_by_compat(changes.changed.iter().map(|(name, c)| (name, *c)));

    let changes_per_symbol = changes
        .changed_by_symbol()
        .into_iter()
        .map(|(symbol, versions)| {
            (
                symbol,
                versions
                    .into_iter()
                    .map(|(version, diff)| (version.clone(), diff.semantic.compat()))
                    .collect(),
            )
        })
        .collect();

    let mut report = JsonHistSummaryReport {
        stable: changes.stable.iter().cloned().collect(),
        unstable: ChangedSymbols {
            breaking: unstable_by_compat
                .remove(&Compatibility::Breaking)
                .unwrap_or_default(),
            dubious: unstable_by_compat
                .remove(&Compatibility::Dubious)
                .unwrap_or_default(),
            compatible: unstable_by_compat
                .remove(&Compatibility::BackwardCompatible)
                .unwrap_or_default(),
        },
        changes_per_version: Vec::with_capacity(changes.changed_by_version.len()),
        changes_per_symbol,
    };

    let paths = &changes.inputs;

    for (version, changes) in changes.changed_by_version.iter() {
        let mut changes_by_compat = changes_by_compat(
            changes
                .iter()
                .map(|(name, diff)| (name, diff.semantic.compat())),
        );
        let summary = HeaderDiffSummary {
            version_old: version.old.to_string(),
            version_new: version.new.to_string(),
            input_old: paths.get(&version.old).unwrap().to_owned(),
            input_new: paths.get(&version.new).unwrap().to_owned(),
            changed: ChangedSymbols {
                breaking: changes_by_compat
                    .remove(&Compatibility::Breaking)
                    .unwrap_or_default(),
                dubious: changes_by_compat
                    .remove(&Compatibility::Dubious)
                    .unwrap_or_default(),
                compatible: changes_by_compat
                    .remove(&Compatibility::BackwardCompatible)
                    .unwrap_or_default(),
            },
        };
        report.changes_per_version.push(summary);
    }

    serde_json::to_writer_pretty(writer, &report).context("json serialisation failed")?;
    Ok(())
}
