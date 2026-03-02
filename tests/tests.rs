use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use ch_diff::{
    ast::{
        HeaderContent,
        c_enum::CEnum,
        c_struct::{CStruct, StructField},
        c_type::{BasicType, CType, SimplifiedTypeKind, StandardIntType},
        c_union::CUnion,
    },
    diff::{ChangeBuf, DiffReport, structs::StructChange},
};
use clang::{Clang, Index};
use pretty_assertions::assert_eq;

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

#[test]
fn parse_types() {
    let clang = Clang::new().unwrap();
    let index = Index::new(&clang, true, true);

    let file = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("inputs/types.h");
    let tu = index.parser(file).parse().unwrap();

    for item in tu.get_entity().get_children() {
        if !item.is_in_main_file() {
            continue;
        }
        let ty = item.get_type();
        println!("- {item:?}: {ty:?}");
        println!("\t sizeof(…): {:?}", ty.unwrap().get_sizeof());
        println!(
            "\t elaborated_type: {:?}",
            ty.unwrap().get_elaborated_type()
        );

        // if item.get_kind() == clang::EntityKind::UnionDecl {
        //     println!("fields: {:?}", ty.unwrap().get_fields().unwrap());
        //     println!("children: {:?}", item.get_children());
        //     println!("fields types: ");
        //     for f in ty.unwrap().get_fields().unwrap() {
        //         let ft = f.get_type().unwrap();
        //         println!("- {}: {ft:?}", f.get_name().unwrap());
        //         println!("  ->elt {:?}", ft.get_elaborated_type());
        //         println!(
        //             "  ->decl {:?}",
        //             ft.get_elaborated_type()
        //                 .unwrap()
        //                 .get_declaration()
        //                 .unwrap()
        //                 .is_anonymous()
        //         );
        //         println!("  => {:?}", CType::try_from(ft));
        //     }
        //     return;
        // }
    }

    let content = HeaderContent::analyse(tu).unwrap();

    // check global variables
    let global_var_names: Vec<_> = content.global_variables.keys().collect();
    let mut expected_var_names = vec![
        "EXTERN_VAR",
        "IMPLICIT",
        "EXPLICIT_ONE",
        "bits32",
        "bits16",
        "bits8",
    ];
    expected_var_names.sort();
    assert_eq!(global_var_names, expected_var_names);
    assert_eq!(
        content
            .global_variables
            .get("EXTERN_VAR")
            .unwrap()
            .payload
            .typ
            .kind,
        SimplifiedTypeKind::Basic(BasicType(clang::TypeKind::Int))
    );
    assert_eq!(
        content
            .global_variables
            .get("IMPLICIT")
            .unwrap()
            .payload
            .typ
            .kind,
        SimplifiedTypeKind::StandardInt(StandardIntType::UIntPtr)
    );
    assert_eq!(
        content
            .global_variables
            .get("EXPLICIT_ONE")
            .unwrap()
            .payload
            .typ
            .kind,
        SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(64))
    );
    assert_eq!(
        content
            .global_variables
            .get("bits32")
            .unwrap()
            .payload
            .typ
            .kind,
        SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(32))
    );
    assert_eq!(
        content
            .global_variables
            .get("bits16")
            .unwrap()
            .payload
            .typ
            .kind,
        SimplifiedTypeKind::StandardInt(StandardIntType::IntFixed(16))
    );
    assert_eq!(
        content
            .global_variables
            .get("bits8")
            .unwrap()
            .payload
            .typ
            .kind,
        SimplifiedTypeKind::StandardInt(StandardIntType::IntFixed(8))
    );

    // structures
    // let struct_names = content.structs.keys().collect::<Vec<_>>();
    // let mut expected_struct_names = vec!["t1"];
    // TODO more tests

    for (var_name, node) in content.structs {
        println!("{var_name}: {node:?}");
    }
    for (var_name, node) in content.unions {
        println!("{var_name}: {node:?}");
    }
    for (var_name, node) in content.enums {
        println!("{var_name}: {node:?}");
    }
}

#[test]
fn diff_functions() {
    let clang = Clang::new().unwrap();
    let index = Index::new(&clang, true, true);

    let file_v1 = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("inputs/diff/structs_v1.h");
    let file_v2 = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("inputs/diff/structs_v2.h");
    let tu1 = index.parser(file_v1).parse().unwrap();
    let tu2 = index.parser(file_v2).parse().unwrap();

    let h1 = HeaderContent::analyse(tu1).unwrap();
    let h2 = HeaderContent::analyse(tu2).unwrap();
    let diff = DiffReport::compute_diff(&h1, &h2).unwrap();
    assert!(diff.global_vars.is_empty());
    assert!(diff.enums.is_empty());
    assert!(!diff.structs.is_empty());
    assert!(diff.functions.is_empty());

    for s in h1.structs {
        println!("{}: {:?}", s.0, s.1);
    }

    // let expected_changes = BTreeMap::from_iter(&[
    //     (String::from("renamed"), ChangeBuf::from_iter([
    //         StructChange::FieldRenamed { old_name: String::from("chr"), new_name: String::from("rch"), field: StructField {
    //             offset: 0,
    //             bit_field_width: None,
    //             typ: CType::B,
    //         } }
    //     ]))
    // ]);
    for (s, sdiff) in diff.structs {
        println!("{s}: {:?}", sdiff.changes);
    }
}
