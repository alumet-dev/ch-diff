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
}

impl ExportedSymbolsDiff {
    pub fn compute_diff(
        a: &HeaderContent,
        b: &HeaderContent,
        filter: &DiffFilter,
    ) -> anyhow::Result<Self> {
        let symbols_a = a.symbols().cloned().collect::<FxHashSet<String>>();
        let symbols_b = b.symbols().cloned().collect::<FxHashSet<String>>();
        let added = symbols_b.difference(&symbols_a);
        let removed = symbols_a.difference(&symbols_b);

        let added = Vec::from_iter(added.filter(|x| filter.accepts(x)).cloned());
        let removed = Vec::from_iter(removed.filter(|x| filter.accepts(x)).cloned());
        Ok(Self { added, removed })
    }

    pub fn compatibility(&self) -> Compatibility {
        if self.removed.is_empty() {
            Compatibility::BackwardCompatible
        } else {
            Compatibility::Breaking
        }
    }
}
