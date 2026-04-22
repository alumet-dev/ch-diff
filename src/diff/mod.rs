use crate::diff::items::{
    enums::EnumDiff, functions::FunctionDiff, opaque::OpaqueDiff, structs::StructDiff,
    unions::UnionDiff, variables::VarChange,
};

pub mod buffer;
pub mod filter;
pub mod items;
pub mod report;

// TODO differentiate source-compatibility and abi-compatibility

#[derive(PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord, Hash, derive_more::Display)]
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
