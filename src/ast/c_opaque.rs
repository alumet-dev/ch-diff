use anyhow::anyhow;
use clang::{Entity, EntityKind};

#[derive(Clone, PartialEq, Debug)]
pub struct OpaqueDecl {
    pub kind: OpaqueDeclKind,
}

#[derive(Clone, PartialEq, Eq, Debug, derive_more::Display)]
pub enum OpaqueDeclKind {
    Struct,
    Enum,
    Union,
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
        Ok(Self { kind })
    }
}
