use std::{collections::BTreeMap, path::PathBuf};

use anyhow::Context;
use clang::Clang;
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    ast::Header,
    diff::{
        Change, Compatibility,
        filter::DiffFilter,
        report::{Diff, DiffReport},
    },
    hist::version::{BiVersion, Version},
};

pub struct ClassifiedChanges {
    pub stable: FxHashSet<String>,
    pub changed: FxHashMap<String, Compatibility>,
    pub changed_by_version: BTreeMap<BiVersion, FxHashMap<String, Diff>>,
    pub inputs: FxHashMap<Version, PathBuf>,
}

impl ClassifiedChanges {
    pub fn changed_by_symbol(&self) -> FxHashMap<String, Vec<(Version, &Diff)>> {
        let mut res: FxHashMap<String, Vec<(Version, &Diff)>> = FxHashMap::default();
        for (version, changes) in &self.changed_by_version {
            for (symbol, diff) in changes {
                res.entry(symbol.to_owned())
                    .or_default()
                    .push((version.new.clone(), diff));
            }
        }
        res
    }
}

pub fn classify_changes_in_history(
    files: &[(PathBuf, Version)],
    clang: &Clang,
    filter: &DiffFilter,
) -> ClassifiedChanges {
    let mut changed = FxHashMap::default();
    let mut changed_by_version = BTreeMap::default();
    let mut stable = None;

    // remember the input paths
    let inputs = files
        .iter()
        .map(|(k, v)| (v.to_owned(), k.to_owned()))
        .collect();

    for ((old_path, old_version), (new_path, new_version)) in files.into_iter().tuple_windows() {
        log::debug!("Comparing {old_path:?} and {new_path:?}");
        let old_header = Header::parse(&clang, &old_path).unwrap();
        let new_header = Header::parse(&clang, &new_path).unwrap();

        // compute diff
        let report = DiffReport::compute_diff(&old_header, &new_header, &filter)
            .with_context(|| {
                format!(
                    "failed to compute diff_path between {:?} and {:?}",
                    old_path.display(),
                    new_path.display()
                )
            })
            .unwrap();

        // initialise the list of stable items
        let stable = stable.get_or_insert_with(|| {
            let mut initial_set = FxHashSet::default();
            for no_changes in report.declarations.unchanged.values() {
                for name in no_changes {
                    initial_set.insert(name.to_owned());
                }
            }
            initial_set
        });

        // update the list of changed and stable items
        let mut diff_here = FxHashMap::default();
        for changes in report.declarations.changed.into_values() {
            for (name, diff) in changes {
                let compat = diff.semantic.compat();
                stable.remove(&name);
                changed
                    .entry(name.to_owned())
                    .and_modify(|c| *c = Ord::min(*c, compat))
                    .or_insert(compat);

                diff_here.insert(name.to_owned(), diff);
            }
        }
        println!("changed in {new_version:?}: {}", diff_here.len());
        changed_by_version.insert(BiVersion::from_ref(old_version, new_version), diff_here);
    }

    ClassifiedChanges {
        stable: stable.unwrap_or_default(),
        changed,
        changed_by_version,
        inputs,
    }
}
