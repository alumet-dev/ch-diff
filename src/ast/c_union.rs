use anyhow::{Context, anyhow};
use clang::{Entity, EntityKind};

use crate::ast::{Node, c_struct::StructField};

#[derive(Debug, PartialEq, Clone)]
pub struct CUnion {
    pub size: usize,
    // it's a union, the fields overlap
    pub fields: Vec<super::Node<StructField>>,
}

impl CUnion {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        assert!(e.get_kind() == EntityKind::UnionDecl);
        if !e.is_definition() {
            return Err(anyhow!(
                "cannot create StructDef the declaration of {}, I need a definition",
                e.get_name().unwrap()
            ));
        }

        let struct_type = e.get_type().unwrap();
        let size = struct_type.get_sizeof().unwrap();

        let children = e.get_children();
        let mut fields = Vec::with_capacity(children.len());
        for child in children {
            if child.get_kind() != EntityKind::FieldDecl {
                continue;
            }

            let field = StructField::try_from_clang(child, struct_type).with_context(|| {
                format!(
                    "failed to parse union field {:?} in {}",
                    child.get_name(),
                    struct_type.get_display_name()
                )
            })?;
            fields.push(super::Node::from_entity(field, &child));
        }

        Ok(Self { size, fields })
    }
}
