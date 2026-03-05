//! Diff of exported symbols.
//! - warning: added symbol
//! - breaking: removed symbol
use rustc_hash::FxHashSet;

use crate::{ast::HeaderContent, diff::ChangeKind};

pub struct ExportedSymbolsDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

impl ExportedSymbolsDiff {
    pub fn compute_diff(a: &HeaderContent, b: &HeaderContent) -> anyhow::Result<Self> {
        let symbols_a = a.symbols().cloned().collect::<FxHashSet<String>>();
        let symbols_b = b.symbols().cloned().collect::<FxHashSet<String>>();
        let added = symbols_b.difference(&symbols_a);
        let removed = symbols_a.difference(&symbols_b);

        let added = Vec::from_iter(added.cloned());
        let removed = Vec::from_iter(removed.cloned());
        Ok(Self { added, removed })
    }

    pub fn compatibility(&self) -> ChangeKind {
        if self.removed.is_empty() {
            ChangeKind::BackwardCompatible
        } else {
            ChangeKind::Breaking
        }
    }
}
