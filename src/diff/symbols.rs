//! Diff of exported symbols.
//! - warning: added symbol
//! - breaking: removed symbol
use rustc_hash::FxHashSet;

use crate::{
    ast::HeaderContent,
    diff::{Change, ChangeBuf, ChangeKind},
};

pub struct ExportedSymbolsDiff {
    pub changes: ChangeBuf<SymbolChange>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SymbolChange {
    Added(String),
    Removed(String),
}

impl Change for SymbolChange {
    fn kind(&self) -> ChangeKind {
        match self {
            SymbolChange::Added(_) => ChangeKind::BackwardCompatible,
            SymbolChange::Removed(_) => ChangeKind::Breaking,
        }
    }
}

impl ExportedSymbolsDiff {
    pub fn compute_diff(a: &HeaderContent, b: &HeaderContent) -> anyhow::Result<Self> {
        let mut changes = ChangeBuf::new();

        let symbols_a = a.symbols().cloned().collect::<FxHashSet<String>>();
        let symbols_b = b.symbols().cloned().collect::<FxHashSet<String>>();
        let added_symbols = symbols_b.difference(&symbols_a);
        let removed_symbols = symbols_a.difference(&symbols_b);

        changes.extend(added_symbols.map(|sym| SymbolChange::Added(sym.to_owned())));
        changes.extend(removed_symbols.map(|sym| SymbolChange::Removed(sym.to_owned())));

        Ok(Self { changes })
    }
}
