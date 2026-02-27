use clang::{Entity, EntityKind};
use indexmap::IndexMap;

use crate::ast::c_type::CType;

#[derive(Debug)]
pub struct CFunction {
    pub arguments: IndexMap<String, FunctionArg>,
    pub return_type: CType,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FunctionArg {
    pub name: String,
    pub typ: CType,
}

impl CFunction {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        assert!(e.get_kind() == EntityKind::FunctionDecl);
        let return_type = e.get_result_type().unwrap().try_into()?;
        let arguments = e
            .get_arguments()
            .unwrap()
            .into_iter()
            .map(|arg| {
                let name = arg.get_name().unwrap_or_default();
                let typ = arg.get_type().unwrap().try_into().unwrap();
                let arg = FunctionArg { name, typ };
                (arg.name.clone(), arg)
            })
            .collect();
        Ok(Self {
            arguments,
            return_type,
        })
    }
}
