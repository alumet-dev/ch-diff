use std::{collections::BTreeMap, fmt::Debug};

use anyhow::Context;

use crate::{
    ast::{
        HeaderContent, Node,
        c_struct::{CStruct, StructField},
    },
    diff::{
        enums::EnumDiff, functions::FunctionDiff, structs::StructDiff,
        symbols::ExportedSymbolsDiff, variables::GlobalVarDiff,
    },
};

pub mod enums;
pub mod functions;
pub mod structs;
pub mod symbols;
pub mod variables;

pub trait Change {
    fn kind(&self) -> ChangeKind;
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord)]
pub enum ChangeKind {
    /// A backward-compatible change, for instance a new function has been added.
    BackwardCompatible,

    /// It might be a breaking change, or not.
    ///
    /// For instance, the name of a struct field has changed.
    /// Human review is necessary to determine whether this change modifies the semantic meaning of the field.
    /// For instance, if the unit of the field has changed, it's a backward-incompatible change.
    Dubious,

    /// A breaking change, for instance a parameter has been added to a function.
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
}

impl<C: Change> FromIterator<C> for ChangeBuf<C> {
    fn from_iter<T: IntoIterator<Item = C>>(iter: T) -> Self {
        let mut res = Self::new();
        res.extend(iter);
        res
    }
}

pub struct DiffReport {
    pub global_vars: GlobalVarDiff,
    pub enums: BTreeMap<String, EnumDiff>,
    pub structs: BTreeMap<String, StructDiff>,
    pub functions: BTreeMap<String, FunctionDiff>,
}

impl DiffReport {
    pub fn compute_diff(a: &HeaderContent, b: &HeaderContent) -> anyhow::Result<DiffReport> {
        // vars
        let global_vars = GlobalVarDiff::compute_diff(a, b)
            .context("fialed to compute the difference in global variables")?;

        // enums
        let mut enums = BTreeMap::new();
        for (name, node) in a.enums.iter() {
            match b.enums.get(name) {
                Some(new_node) => {
                    let diff = EnumDiff::compute_diff(&node.payload, &new_node.payload)?;
                    enums.insert(name.to_owned(), diff);
                }
                None => {
                    // enum removed
                }
            }
        }

        // structs
        let mut structs = BTreeMap::new();
        for (name, node) in a.structs.iter() {
            match b.structs.get(name) {
                Some(new_node) => {
                    let diff = StructDiff::compute_diff(&node.payload, &new_node.payload)?;
                    structs.insert(name.to_owned(), diff);
                }
                None => {
                    // function removed
                }
            }
        }

        // functions
        let mut functions = BTreeMap::new();
        for (name, node) in a.functions.iter() {
            match b.functions.get(name) {
                Some(new_node) => {
                    let diff = FunctionDiff::compute_diff(&node.payload, &new_node.payload)?;
                    functions.insert(name.to_owned(), diff);
                }
                None => {
                    // struct removed
                }
            }
        }

        Ok(DiffReport {
            global_vars,
            enums,
            structs,
            functions,
        })
    }

    pub fn global_compatibility(&self) -> ChangeKind {
        let mut compat = self.global_vars.changes.compatibility;
        if let Some(enum_compat) = self
            .enums
            .values()
            .map(|diff| diff.changes.compatibility)
            .max()
        {
            compat = compat.max(enum_compat);
        }
        if let Some(struct_compat) = self
            .structs
            .values()
            .map(|diff| diff.changes.compatibility)
            .max()
        {
            compat = compat.max(struct_compat);
        }
        if let Some(func_compat) = self
            .functions
            .values()
            .map(|diff| diff.changes.compatibility)
            .max()
        {
            compat = compat.max(func_compat);
        }
        compat
    }
}
