use clang::{Entity, EntityKind};

use crate::ast::c_type::CType;

#[derive(Debug)]
pub struct CFunction {
    // by position in the argument list
    pub arguments: Vec<FunctionArg>,
    pub return_type: CType,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunctionArg {
    pub name: String,
    pub typ: CType,
    pub pos: usize,
}

impl CFunction {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        assert!(e.get_kind() == EntityKind::FunctionDecl);
        let return_type = e.get_result_type().unwrap().try_into()?;
        let arguments = e
            .get_arguments()
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(position, arg)| {
                let name = arg.get_name().unwrap_or_default();
                let typ = arg.get_type().unwrap().try_into().unwrap();
                FunctionArg {
                    name,
                    typ,
                    pos: position,
                }
            })
            .collect();
        Ok(Self {
            arguments,
            return_type,
        })
    }
}
