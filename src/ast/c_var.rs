use clang::{Entity, EntityKind};

use crate::ast::c_type::CType;

#[derive(Debug)]
pub struct CVar {
    pub typ: CType,
    pub is_invalid: bool,
}

impl CVar {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        assert!(e.get_kind() == EntityKind::VarDecl);
        let typ = e.get_type().unwrap().try_into()?;
        let is_invalid = e.is_invalid_declaration();
        Ok(Self { typ, is_invalid })
    }
}
