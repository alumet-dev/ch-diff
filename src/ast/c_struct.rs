use std::collections::BTreeMap;

use anyhow::{Context, anyhow};
use clang::{Entity, EntityKind, Type};

use crate::ast::c_type::{CType, anon::AnonContext};

#[derive(Debug, PartialEq, Clone)]
pub struct CStruct {
    /// Size of the struct in bytes.
    pub size: usize,

    /// Fields by offset, because renaming a field is not a breaking change (for the ABI).
    pub fields: BTreeMap<usize, super::Node<StructField>>,

    /// Definitions of anonymous types.
    pub anonymous: AnonContext,
}

/// Information about a struct field, **without its name**.
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub offset: usize,

    /// If this is a bit-field, its number of bits.
    ///
    /// A bit-field is declared like this:
    /// ```c
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
                "cannot create StructDef from the declaration of {}, I need a definition",
                e.get_name().unwrap()
            ));
        }

        let struct_type = e.get_type().unwrap();
        let size = struct_type.get_sizeof().unwrap();
        let mut anonymous = AnonContext::new();

        let children = e.get_children();
        let mut fields = BTreeMap::new();
        for child in children {
            if child.get_kind() != EntityKind::FieldDecl {
                continue;
            }

            let field = StructField::try_from_clang(child, struct_type, &mut anonymous)
                .with_context(|| {
                    format!(
                        "failed to parse struct field {:?} in {}",
                        child.get_name(),
                        struct_type.get_display_name()
                    )
                })?;
            fields.insert(field.offset, super::Node::from_entity(field, &child));
        }

        Ok(Self {
            size,
            fields,
            anonymous,
        })
    }
}

impl StructField {
    pub fn try_from_clang<'a>(
        item: Entity<'a>,
        parent_type: Type<'a>,
        parent_ctx: &mut AnonContext,
    ) -> anyhow::Result<Self> {
        assert!(item.get_kind() == EntityKind::FieldDecl);

        // we have a field declaration
        let name = item.get_name().unwrap_or_default();
        let offset = parent_type.get_offsetof(&name).unwrap();
        let typ = item.get_type().unwrap();
        let typ = CType::try_from_clang(typ, Some(parent_ctx))?;

        // it may be a bit-field
        let bit_field_width = item.get_bit_field_width();

        Ok(StructField {
            offset,
            bit_field_width,
            typ,
        })
    }
}
