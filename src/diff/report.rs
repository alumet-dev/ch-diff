use std::{collections::BTreeMap, marker::PhantomData};

use anyhow::Context;
use enum_map::EnumMap;

use crate::{
    ast::{Header, Node},
    diff::{
        Change, Compatibility, DeclDiff, DeclKind, SemanticDiff, SourceDiff, SourceDiffStyle,
        filter::DiffFilter,
        items::{
            enums::{self, EnumDiff},
            functions::FunctionDiff,
            opaque::OpaqueDiff,
            structs::StructDiff,
            symbols::ExportedSymbolsDiff,
            unions::UnionDiff,
            variables::VarChange,
        },
    },
};

pub struct Diff {
    pub semantic: SemanticDiff,
    pub source: SourceDiff,
}

/// Reports the differences between two C headers.
pub struct DiffReport {
    pub old_name: String,
    pub new_name: String,
    pub declarations: EnumMap<DeclKind, BTreeMap<String, Diff>>,
    pub symbols: ExportedSymbolsDiff,
}

impl DiffReport {
    /// Compute the difference between two headers.
    pub fn compute_diff(a: &Header, b: &Header, filter: DiffFilter) -> anyhow::Result<DiffReport> {
        let old_name = a
            .file
            .file_name()
            .map(|s| s.to_str().unwrap())
            .unwrap_or("new");
        let new_name = b
            .file
            .file_name()
            .map(|s| s.to_str().unwrap())
            .unwrap_or("old");
        let a = &a.content;
        let b = &b.content;

        let mut declarations: EnumMap<DeclKind, BTreeMap<String, _>> = EnumMap::default();

        // global variables
        NodeMapDiffer::builder()
            .filter(&filter)
            .sourcer(commented_source_code)
            .differ(VarChange::compute_diff)
            .on_change(|name, diff| {
                declarations[DeclKind::GlobalVar].insert(name.to_owned(), diff);
            })
            .build()
            .find_differences(&a.global_variables, &b.global_variables)
            .context("could not compute the difference in global variables")?;

        // enums
        NodeMapDiffer::builder()
            .filter(&filter)
            .sourcer(enums::normalized_source_code)
            .differ(EnumDiff::semantic_diff)
            .on_change(|name, diff| {
                declarations[DeclKind::Enum].insert(name.to_owned(), diff);
            })
            .build()
            .find_differences(&a.enums, &b.enums)
            .context("could not compute the difference in enums")?;

        // structs
        NodeMapDiffer::builder()
            .filter(&filter)
            .sourcer(commented_source_code)
            .differ(StructDiff::compute_diff)
            .on_change(|name, diff| {
                declarations[DeclKind::Struct].insert(name.to_owned(), diff);
            })
            .build()
            .find_differences(&a.structs, &b.structs)
            .context("could not compute the difference in structs")?;

        // unions
        NodeMapDiffer::builder()
            .filter(&filter)
            .sourcer(commented_source_code)
            .differ(UnionDiff::compute_diff)
            .on_change(|name, diff| {
                declarations[DeclKind::Union].insert(name.to_owned(), diff);
            })
            .build()
            .find_differences(&a.unions, &b.unions)
            .context("could not compute the difference in unions")?;

        // functions
        NodeMapDiffer::builder()
            .filter(&filter)
            .sourcer(commented_source_code)
            .source_diff_style(SourceDiffStyle::Split1v1)
            .differ(FunctionDiff::compute_diff)
            .on_change(|name, diff| {
                declarations[DeclKind::Function].insert(name.to_owned(), diff);
            })
            .build()
            .find_differences(&a.functions, &b.functions)
            .context("could not compute the difference in functions")?;

        // opaque declarations
        NodeMapDiffer::builder()
            .filter(&filter)
            .sourcer(commented_source_code)
            .differ(OpaqueDiff::compute_diff)
            .on_change(|name, diff| {
                declarations[DeclKind::Opaque].insert(name.to_owned(), diff);
            })
            .build()
            .find_differences(&a.opaques, &b.opaques)
            .context("could not compute the difference in opaque declarations")?;

        // public symbols (functions and variables)
        let symbols = ExportedSymbolsDiff::compute_diff(a, b, &filter)
            .context("failed to compute ExportedSymbolsDiff")?;

        Ok(DiffReport {
            old_name: old_name.to_owned(),
            new_name: new_name.to_owned(),
            declarations,
            symbols,
        })
    }

    pub fn global_compatibility(&self) -> Compatibility {
        let mut compat = self.symbols.compatibility();

        if let Some(c) = self
            .declarations
            .values()
            .map(|d| d.values().map(|diff| diff.semantic.compat()).min())
            .min()
            .flatten()
        {
            compat = compat.min(c);
        }
        compat
    }
}

/// Helper to compute the difference between two `BTreeMap<String, Node<N>>`.
/// See [find_differences](Self::find_differences).
#[derive(bon::Builder)]
struct NodeMapDiffer<'d, N, D, R, F, S>
where
    D: FnMut(&N, &N) -> anyhow::Result<Option<R>>,
    R: Into<DeclDiff>,
    F: FnMut(&str, Diff),
    S: for<'a> FnMut(&'a Node<N>) -> String,
{
    filter: &'d DiffFilter,

    /// Function that computes the difference between two items `N`.
    /// Returns `None` when there is no difference.
    differ: D,

    /// Function that returns the formatted source code of a node.
    sourcer: S,

    /// Callback to call on each change.
    on_change: F,

    #[builder(default)]
    source_diff_style: SourceDiffStyle,

    #[builder(skip)]
    _phantom: PhantomData<N>,
}

impl<'d, N, D, R, F, S> NodeMapDiffer<'d, N, D, R, F, S>
where
    D: FnMut(&N, &N) -> anyhow::Result<Option<R>>,
    R: Into<DeclDiff>,
    F: FnMut(&str, Diff),
    S: for<'a> FnMut(&'a Node<N>) -> String,
{
    fn find_differences(
        &mut self,
        a: &BTreeMap<String, Node<N>>,
        b: &BTreeMap<String, Node<N>>,
    ) -> anyhow::Result<()> {
        // check existing nodes
        for (name, node_a) in a {
            if !self.filter.accepts(name) {
                continue;
            }
            let source_old = (self.sourcer)(node_a);
            match b.get(name) {
                Some(node_b) => {
                    if let Some(diff) = (self.differ)(&node_a.payload, &node_b.payload)? {
                        let source_new = (self.sourcer)(node_b);
                        let diff = Diff {
                            semantic: SemanticDiff::Modified(diff.into()),
                            source: SourceDiff {
                                old: source_old,
                                new: source_new,
                                style: self.source_diff_style,
                            },
                        };
                        (self.on_change)(name, diff)
                    }
                }
                None => {
                    let diff = Diff {
                        semantic: SemanticDiff::Removed,
                        source: SourceDiff {
                            old: source_old,
                            new: String::new(),
                            style: self.source_diff_style,
                        },
                    };
                    (self.on_change)(name, diff)
                }
            }
        }

        // check new nodes
        for (name, node_b) in b {
            if !self.filter.accepts(name) {
                continue;
            }
            if !a.contains_key(name) {
                let source_new = (self.sourcer)(node_b);
                let diff = Diff {
                    semantic: SemanticDiff::Removed,
                    source: SourceDiff {
                        old: String::new(),
                        new: source_new,
                        style: self.source_diff_style,
                    },
                };
                (self.on_change)(name, diff);
            }
        }
        Ok(())
    }
}

fn commented_source_code<N>(n: &Node<N>) -> String {
    if n.comment.is_empty() {
        n.source_code.to_owned()
    } else {
        format!("/* {} */\n{}", n.comment, n.source_code)
    }
}
