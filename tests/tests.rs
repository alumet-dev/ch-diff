use std::path::PathBuf;

use ch_diff::{
    ast::{
        Header, HeaderContent,
        c_opaque::OpaqueDeclKind,
        c_type::{BasicType, SimplifiedTypeKind, stdint::StandardIntType},
    },
    diff::{DeclKind, filter::DiffFilter, report::DiffReport},
};
use clang::{Clang, EntityKind, Index};
use pretty_assertions::assert_eq;
use serial_test::serial;

#[test]
#[serial]
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
#[serial]
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
        SimplifiedTypeKind::OtherBasic(BasicType(clang::TypeKind::Int))
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
#[serial]
fn diff_structs() {
    let clang = Clang::new().unwrap();

    let file_v1 = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("inputs/diff/structs_v1.h");
    let file_v2 = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("inputs/diff/structs_v2.h");
    let h1 = Header::parse(&clang, file_v1).unwrap();
    let h2 = Header::parse(&clang, file_v2).unwrap();
    let filter = DiffFilter::allow_everything();
    let diff = DiffReport::compute_diff(&h1, &h2, &filter).unwrap();
    assert!(diff.declarations.changed[DeclKind::GlobalVar].is_empty());
    assert!(diff.declarations.changed[DeclKind::Function].is_empty());
    assert!(diff.declarations.changed[DeclKind::Enum].is_empty());
    assert!(!diff.declarations.changed[DeclKind::Struct].is_empty());
    assert!(diff.declarations.changed[DeclKind::Union].is_empty());
    assert!(diff.declarations.changed[DeclKind::Opaque].is_empty());

    for s in h1.content.structs {
        println!("{}: {:?}", s.0, s.1);
    }
}

#[test]
#[serial]
fn diff_structs_hard() {
    let clang = Clang::new().unwrap();

    let file_v1 = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("inputs/diff/structs_hard_v1.h");
    let file_v2 = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("inputs/diff/structs_hard_v2.h");
    let h1 = Header::parse(&clang, file_v1).unwrap();
    let h2 = Header::parse(&clang, file_v2).unwrap();
    let filter = DiffFilter::allow_everything();
    let diff = DiffReport::compute_diff(&h1, &h2, &filter).unwrap();
    assert!(diff.declarations.changed[DeclKind::GlobalVar].is_empty());
    assert!(diff.declarations.changed[DeclKind::Enum].is_empty());
    assert!(!diff.declarations.changed[DeclKind::Struct].is_empty());
    assert!(diff.declarations.changed[DeclKind::Function].is_empty());

    for (name, cstruct) in h1.content.structs {
        println!("\n{name} (size = {})", cstruct.payload.size);
        for (offset, field) in cstruct.payload.fields {
            println!("- offset {offset}: {field:?}");
        }
    }

    for (name, cstruct) in h2.content.structs {
        println!("\n{name} (size = {})", cstruct.payload.size);
        for (offset, field) in cstruct.payload.fields {
            println!("- offset {offset}: {field:?}");
        }
    }
}

#[test]
#[serial]
fn parse_opaque_structs() {
    let clang = Clang::new().unwrap();
    let index = Index::new(&clang, true, true);

    let file = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("inputs/opaque.h");
    let tu = index.parser(file).parse().unwrap();

    for e in tu.get_entity().get_children() {
        if !e.is_in_main_file() {
            continue;
        }
        let s = e.get_pretty_printer().print();
        println!("{s}");
        println!(
            "is_def={}, is_decl={}",
            e.is_definition(),
            e.is_declaration()
        );
        println!("is_anon_rdecl={}", e.is_anonymous_record_decl());
        println!("is_anon={}", e.is_anonymous());
        if e.get_kind() == EntityKind::EnumDecl {
            println!("underlying type: {:?}", e.get_enum_underlying_type());
        }
        println!("def={:?}", e.get_definition());
        println!("");
    }

    let content = HeaderContent::analyse(tu).unwrap();
    let mut expected_opaques = vec![
        ("foo_".to_owned(), OpaqueDeclKind::Struct),
        ("opaque_struct".to_owned(), OpaqueDeclKind::Struct),
        ("opaque".to_owned(), OpaqueDeclKind::Struct),
        ("opaque_enum".to_owned(), OpaqueDeclKind::Enum),
        ("opaque_union".to_owned(), OpaqueDeclKind::Union),
    ];
    expected_opaques.sort_by_key(|x| x.0.clone());
    let mut actual_opaque = content
        .opaques
        .iter()
        .map(|(name, decl)| (name.to_owned(), decl.payload.kind.to_owned()))
        .collect::<Vec<_>>();
    actual_opaque.sort_by_key(|x| x.0.clone());
    assert_eq!(expected_opaques, actual_opaque);
}
