use std::fmt::Display;

use clang::{Entity, EntityKind};

use crate::ast::c_type::CType;

#[derive(Debug, Clone, PartialEq)]
pub struct CVar {
    pub typ: CType,
    pub is_invalid: bool,
    display: String,
}

impl Display for CVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display)
    }
}

impl CVar {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        assert!(e.get_kind() == EntityKind::VarDecl);
        let typ = e.get_type().unwrap().try_into()?;
        let is_invalid = e.is_invalid_declaration();
        let display = e.get_pretty_printer().print();
        Ok(Self {
            typ,
            is_invalid,
            display,
        })
    }
}
