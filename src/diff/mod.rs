use crate::diff::items::{
    enums::EnumDiff, functions::FunctionDiff, opaque::OpaqueDiff, structs::StructDiff,
    unions::UnionDiff, variables::VarChange,
};

pub mod filter;
pub mod items;
pub mod report;

// TODO differentiate source-compatibility and abi-compatibility

#[derive(PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord, derive_more::Display)]
pub enum Compatibility {
    /// A breaking change, for instance a parameter has been added to a function.
    #[display("breaking change(s)")]
    Breaking,

    /// It might be a breaking change, or not.
    ///
    /// For instance, the name of a struct field has changed.
    /// Human review is necessary to determine whether this change modifies the semantic meaning of the field.
    /// For instance, if the unit of the field has changed, it's a backward-incompatible change.
    #[display("dubious, human verification required")]
    Dubious,

    /// A backward-compatible change, for instance a new function has been added.
    #[display("backward-compatible")]
    BackwardCompatible,
}

pub trait Change {
    fn compat(&self) -> Compatibility;
}

#[derive(Debug)]
pub struct ChangeBuf<C: Change> {
    changes: Vec<C>,
    compatibility: Compatibility,
}

impl<C: Change> ChangeBuf<C> {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            compatibility: Compatibility::BackwardCompatible,
        }
    }

    pub fn push(&mut self, change: C) {
        self.compatibility = self.compatibility.max(change.compat());
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

    pub fn compatibility(&self) -> Compatibility {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, enum_map::Enum)]
pub enum DeclKind {
    GlobalVar,
    Function,
    Enum,
    Struct,
    Union,
    Opaque,
}

pub enum SemanticDiff {
    Added,
    Removed,
    Modified(DeclDiff),
}

impl Change for SemanticDiff {
    fn compat(&self) -> Compatibility {
        match self {
            SemanticDiff::Added => Compatibility::BackwardCompatible,
            SemanticDiff::Removed => Compatibility::Breaking,
            SemanticDiff::Modified(c) => c.compat(),
        }
    }
}

#[derive(derive_more::From)]
pub enum DeclDiff {
    GlobalVar(VarChange),
    Enum(EnumDiff),
    Struct(StructDiff),
    Union(UnionDiff),
    Function(FunctionDiff),
    Opaque(OpaqueDiff),
}

impl Change for DeclDiff {
    fn compat(&self) -> Compatibility {
        match self {
            DeclDiff::GlobalVar(diff) => diff.compat(),
            DeclDiff::Enum(diff) => diff.compat(),
            DeclDiff::Struct(diff) => diff.compat(),
            DeclDiff::Union(diff) => diff.compat(),
            DeclDiff::Function(diff) => diff.compat(),
            DeclDiff::Opaque(diff) => diff.compat(),
        }
    }
}

pub struct SourceDiff {
    pub old: String,
    pub new: String,
    pub style: SourceDiffStyle,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SourceDiffStyle {
    Multiline,
    Split1v1,
}

impl Default for SourceDiffStyle {
    fn default() -> Self {
        Self::Multiline
    }
}
