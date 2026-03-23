use std::fmt::Display;

use anyhow::anyhow;
use clang::{Entity, EntityKind};

#[derive(Clone, PartialEq, Debug)]
pub struct OpaqueDecl {
    display: String,
    pub kind: OpaqueDeclKind,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OpaqueDeclKind {
    Struct,
    Enum,
    Union,
}

impl Display for OpaqueDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display)
    }
}

impl OpaqueDecl {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        assert!(
            !e.is_definition(),
            "definition are not opaque, they should not arrive here"
        );
        let kind = match e.get_kind() {
            EntityKind::StructDecl => OpaqueDeclKind::Struct,
            EntityKind::EnumDecl => OpaqueDeclKind::Enum,
            EntityKind::UnionDecl => OpaqueDeclKind::Union,
            _ => return Err(anyhow!("invalid entity for OpaqueDecl: {:?}", e.get_kind())),
        };
        let display = e.get_pretty_printer().print();
        Ok(Self { display, kind })
    }
}
