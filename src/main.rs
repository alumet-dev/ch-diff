use std::collections::BTreeMap;

use anyhow::anyhow;
use clang::*;
use indexmap::IndexMap;

use crate::ast::{
    Node, c_enum::CEnum, c_function::CFunction, c_struct::CStruct, c_type::AliasType, c_var::CVar,
};

mod ast;
mod diff;

fn main() {
    // Acquire an instance of `Clang`
    let clang = Clang::new().unwrap();

    // Create a new `Index`
    let index = Index::new(&clang, true, true);

    // Parse a source file into a translation unit
    // let tu = index.parser("amdsmi.h").parse().unwrap();
    let tu = index.parser("tests/functions.h").parse().unwrap();

    for e in tu.get_entity().get_children() {
        let s = e.get_pretty_printer().print();
        println!("{s}");
    }
}
