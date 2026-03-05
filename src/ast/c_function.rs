use std::fmt::Display;

use clang::{Entity, EntityKind};

use crate::ast::c_type::CType;

#[derive(Debug)]
pub struct CFunction {
    // by position in the argument list
    pub arguments: Vec<FunctionArg>,
    pub return_type: CType,
    display: String,
}

impl Display for CFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display)
    }
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
        let display = e.get_pretty_printer().print();
        Ok(Self {
            arguments,
            return_type,
            display,
        })
    }
}
