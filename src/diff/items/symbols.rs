//! Diff of exported symbols.
//! - warning: added symbol
//! - breaking: removed symbol
use rustc_hash::FxHashSet;

use crate::{
    ast::HeaderContent,
    diff::{Compatibility, filter::DiffFilter},
};

pub struct ExportedSymbolsDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub common: Vec<String>,
}

impl ExportedSymbolsDiff {
    pub fn compute_diff(
        a: &HeaderContent,
        b: &HeaderContent,
        filter: &DiffFilter,
    ) -> anyhow::Result<Self> {
        let symbols_a = a
            .symbols()
            .filter(|x| filter.accepts(x))
            .cloned()
            .collect::<FxHashSet<String>>();
        let symbols_b = b
            .symbols()
            .filter(|x| filter.accepts(x))
            .cloned()
            .collect::<FxHashSet<String>>();

        let added = symbols_b.difference(&symbols_a);
        let removed = symbols_a.difference(&symbols_b);
        let common = symbols_a.intersection(&symbols_b);

        let added = Vec::from_iter(added.cloned());
        let removed = Vec::from_iter(removed.cloned());
        let common = Vec::from_iter(common.cloned());
        Ok(Self {
            added,
            removed,
            common,
        })
    }

    pub fn compatibility(&self) -> Compatibility {
        if self.removed.is_empty() {
            Compatibility::BackwardCompatible
        } else {
            Compatibility::Breaking
        }
    }
}
