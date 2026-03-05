use std::{collections::BTreeMap, fmt::Debug};

use anyhow::Context;

use crate::{
    ast::HeaderContent,
    diff::{
        enums::EnumDiff, functions::FunctionDiff, structs::StructDiff,
        symbols::ExportedSymbolsDiff, unions::UnionDiff, variables::GlobalVarDiff,
    },
};

pub mod enums;
pub mod functions;
pub mod structs;
pub mod symbols;
pub mod unions;
pub mod variables;

pub trait ChangeContainer {
    fn overall_kind(&self) -> ChangeKind;
}

pub trait Change {
    fn kind(&self) -> ChangeKind;
}

// TODO differentiate source-compatibility and abi-compatibility

#[derive(PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord, derive_more::Display)]
pub enum ChangeKind {
    /// A backward-compatible change, for instance a new function has been added.
    #[display("backward-compatible")]
    BackwardCompatible,

    /// It might be a breaking change, or not.
    ///
    /// For instance, the name of a struct field has changed.
    /// Human review is necessary to determine whether this change modifies the semantic meaning of the field.
    /// For instance, if the unit of the field has changed, it's a backward-incompatible change.
    #[display("dubious, human verification required")]
    Dubious,

    /// A breaking change, for instance a parameter has been added to a function.
    #[display("breaking change(s)")]
    Breaking,
}

#[derive(Debug)]
pub struct ChangeBuf<C: Change> {
    changes: Vec<C>,
    compatibility: ChangeKind,
}

impl<C: Change> ChangeBuf<C> {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            compatibility: ChangeKind::BackwardCompatible,
        }
    }

    pub fn push(&mut self, change: C) {
        self.compatibility = self.compatibility.max(change.kind());
        self.changes.push(change);
    }

    pub fn extend<T: IntoIterator<Item = C>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        self.changes.reserve_exact(iter.size_hint().0);
        for change in iter {
            self.push(change);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn compatibility(&self) -> ChangeKind {
        self.compatibility
    }
}

impl<C: Change> IntoIterator for ChangeBuf<C> {
    type Item = C;
    type IntoIter = std::vec::IntoIter<C>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.into_iter()
    }
}

impl<'a, C: Change> IntoIterator for &'a ChangeBuf<C> {
    type Item = &'a C;
    type IntoIter = std::slice::Iter<'a, C>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.iter()
    }
}

impl<C: Change> FromIterator<C> for ChangeBuf<C> {
    fn from_iter<T: IntoIterator<Item = C>>(iter: T) -> Self {
        let mut res = Self::new();
        res.extend(iter);
        res
    }
}

pub struct DiffReport {
    pub old_name: String,
    pub new_name: String,
    pub global_vars: GlobalVarDiff,
    pub enums: BTreeMap<String, DeclChange<EnumDiff>>,
    pub structs: BTreeMap<String, DeclChange<StructDiff>>,
    pub unions: BTreeMap<String, DeclChange<UnionDiff>>,
    pub functions: BTreeMap<String, DeclChange<FunctionDiff>>,
    pub symbols: ExportedSymbolsDiff,
}

#[derive(Debug, Clone)]
pub enum DeclChange<Diff: ChangeContainer> {
    Removed,
    Changed(Diff),
}

impl<Diff: ChangeContainer> DeclChange<Diff> {
    fn compatibility(&self) -> ChangeKind {
        match self {
            DeclChange::Removed => ChangeKind::Breaking,
            DeclChange::Changed(diff) => diff.overall_kind(),
        }
    }
}

impl DiffReport {
    pub fn compute_diff(
        a: (&str, &HeaderContent),
        b: (&str, &HeaderContent),
    ) -> anyhow::Result<DiffReport> {
        let (old_name, a) = a;
        let (new_name, b) = b;

        // vars
        let global_vars = GlobalVarDiff::compute_diff(a, b)
            .context("could not compute the difference in global variables")?;

        // enums
        let mut enums = BTreeMap::new();
        for (name, node) in a.enums.iter() {
            match b.enums.get(name) {
                Some(new_node) => {
                    if let Some(diff) = EnumDiff::compute_diff(&node.payload, &new_node.payload)? {
                        enums.insert(name.to_owned(), DeclChange::Changed(diff));
                    }
                }
                None => {
                    enums.insert(name.to_owned(), DeclChange::Removed);
                }
            };
        }

        // structs
        let mut structs = BTreeMap::new();
        for (name, node) in a.structs.iter() {
            match b.structs.get(name) {
                Some(new_node) => {
                    if let Some(diff) = StructDiff::compute_diff(&node.payload, &new_node.payload)?
                    {
                        structs.insert(name.to_owned(), DeclChange::Changed(diff));
                    }
                }
                None => {
                    structs.insert(name.to_owned(), DeclChange::Removed);
                }
            }
        }

        // unions
        let mut unions = BTreeMap::new();
        for (name, node) in a.unions.iter() {
            match b.unions.get(name) {
                Some(new_node) => {
                    if let Some(diff) = UnionDiff::compute_diff(&node.payload, &new_node.payload)? {
                        unions.insert(name.to_owned(), DeclChange::Changed(diff));
                    }
                }
                None => {
                    unions.insert(name.to_owned(), DeclChange::Removed);
                }
            }
        }

        // functions
        let mut functions = BTreeMap::new();
        for (name, node) in a.functions.iter() {
            let change = match b.functions.get(name) {
                Some(new_node) => {
                    if let Some(diff) =
                        FunctionDiff::compute_diff(&node.payload, &new_node.payload)?
                    {
                        functions.insert(name.to_owned(), DeclChange::Changed(diff));
                    }
                }
                None => {
                    functions.insert(name.to_owned(), DeclChange::Removed);
                }
            };
        }

        // public symbols (functions and variables)
        let symbols = ExportedSymbolsDiff::compute_diff(a, b)
            .context("failed to compute ExportedSymbolsDiff")?;

        Ok(DiffReport {
            old_name: old_name.to_owned(),
            new_name: new_name.to_owned(),
            global_vars,
            enums,
            structs,
            unions,
            functions,
            symbols,
        })
    }

    pub fn global_compatibility(&self) -> ChangeKind {
        let mut compat = self.global_vars.changes.compatibility;

        if let Some(enum_compat) = self.enums.values().map(|d| d.compatibility()).max() {
            compat = compat.max(enum_compat);
        }
        if let Some(struct_compat) = self.structs.values().map(|d| d.compatibility()).max() {
            compat = compat.max(struct_compat);
        }
        if let Some(union_compat) = self.unions.values().map(|d| d.compatibility()).max() {
            compat = compat.max(union_compat);
        }
        if let Some(fn_compat) = self.functions.values().map(|d| d.compatibility()).max() {
            compat = compat.max(fn_compat); // semantically, the order should be reversed and we should use .min()
        }
        compat
    }
}

pub struct SourceDiff {
    pub old: String,
    pub new: String,
}
