use std::path::PathBuf;

use clang::{Clang, Index};

#[test]
fn parse_functions() {
    let clang = Clang::new().unwrap();
    let index = Index::new(&clang, true, true);

    let file = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("inputs/functions.h");
    let tu = index.parser(file).parse().unwrap();

    for e in tu.get_entity().get_children() {
        if !e.is_in_main_file() {
            continue;
        }
        let s = e.get_pretty_printer().print();
        println!("{s}");
    }
}
