use anyhow::anyhow;
use clang::{Entity, EntityKind};
use indexmap::IndexMap;

#[derive(Debug)]
pub struct CStruct {
    pub size: usize,
    pub fields: IndexMap<String, super::Node<StructField>>,
}

#[derive(Debug)]
pub struct StructField {
    pub size: usize,
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

        let fields = e
            .get_children()
            .into_iter()
            .filter_map(|item| {
                if item.get_kind() != EntityKind::FieldDecl {
                    return None;
                }

                // we have a field declaration
                let name = item.get_name().unwrap_or_default();
                let offset = struct_type.get_offsetof(&name).unwrap();
                let size = item.get_type().unwrap().get_sizeof().unwrap();

                // it may be a bit-field
                let bit_field_width = item.get_bit_field_width();

                let field = StructField {
                    size,
                    offset,
                    bit_field_width,
                };
                let field = super::Node::from_entity(field, &item);
                Some((name, field))
            })
            .collect();

        Ok(Self { size, fields })
    }
}
