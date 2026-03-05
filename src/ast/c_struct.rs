use std::{collections::BTreeMap, fmt::Display};

use anyhow::{Context, anyhow};
use clang::{Entity, EntityKind, Type};

use crate::ast::c_type::CType;

#[derive(Debug, PartialEq, Clone)]
pub struct CStruct {
    pub size: usize,
    // store the field by offset: renaming a field is not a breaking change
    pub fields: BTreeMap<usize, super::Node<StructField>>,
    display: String,
}

impl Display for CStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display)
    }
}

/// Information about a struct field, **without its name**.
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub offset: usize,

    /// If this is a bit-field, its number of bits.
    ///
    /// A bit-field is declared like this:
    /// ```
    /// struct S {
    ///     int some_bits: 3;
    ///     int more_bits: 5;
    /// }
    /// ```
    /// See https://en.cppreference.com/w/c/language/bit_field.html
    pub bit_field_width: Option<usize>,

    pub typ: CType,
}

impl CStruct {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        assert!(e.get_kind() == EntityKind::StructDecl);
        if !e.is_definition() {
            return Err(anyhow!(
                "cannot create StructDef the declaration of {}, I need a definition",
                e.get_name().unwrap()
            ));
        }

        let struct_type = e.get_type().unwrap();
        let size = struct_type.get_sizeof().unwrap();

        let children = e.get_children();
        let mut fields = BTreeMap::new();
        for child in children {
            if child.get_kind() != EntityKind::FieldDecl {
                continue;
            }

            let field = StructField::try_from_clang(child, struct_type).with_context(|| {
                format!(
                    "failed to parse struct field {:?} in {}",
                    child.get_name(),
                    struct_type.get_display_name()
                )
            })?;
            fields.insert(field.offset, super::Node::from_entity(field, &child));
        }

        let display = e.get_pretty_printer().print();
        Ok(Self {
            size,
            fields,
            display,
        })
    }
}

impl StructField {
    pub fn try_from_clang<'a>(item: Entity<'a>, record_type: Type<'a>) -> anyhow::Result<Self> {
        assert!(item.get_kind() == EntityKind::FieldDecl);

        // we have a field declaration
        let name = item.get_name().unwrap_or_default();
        let offset = record_type.get_offsetof(&name).unwrap();
        let typ = item.get_type().unwrap().try_into().unwrap();

        // it may be a bit-field
        let bit_field_width = item.get_bit_field_width();

        Ok(StructField {
            offset,
            bit_field_width,
            typ,
        })
    }
}
