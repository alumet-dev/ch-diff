use rustc_hash::{FxHashMap, FxHashSet};

use crate::ast::HeaderContent;

pub struct DiffReport {
    pub added_symbols: Vec<String>,
    pub removed_symbols: Vec<String>,

    pub changed_structs: Vec<()>,
    pub changed_enums: Vec<()>,
    pub changed_functions: Vec<()>,
}

pub struct StructDiff {}

pub struct EnumDiff {}

pub struct FunctionDiff {}

pub fn compute_diff(a: &HeaderContent, b: &HeaderContent) -> anyhow::Result<DiffReport> {
    // exported symbols
    // - warning: added symbol
    // - breaking: removed symbol
    let symbols_a = a.symbols().cloned().collect::<FxHashSet<String>>();
    let symbols_b = b.symbols().cloned().collect::<FxHashSet<String>>();
    let added_symbols = symbols_b.difference(&symbols_a);
    let removed_symbols = symbols_a.difference(&symbols_b);

    // structures
    // - warning: field renamed, typedef change but equivalent underlying type
    // - breaking: field removed, field added, field change (same name, different type)

    // functions
    // - warning: parameter renamed, typedef change but equivalent underlying type
    // - breaking: return type change, parameter removed, parameter added, parameter change (same name, different type)

    // enums:
    // - warning: new values added, values removed (same value but different name)
    // - breaking: values removed (underlying integer value no longer present in enum), values changed (same name, different integer value)
    todo!()
}
