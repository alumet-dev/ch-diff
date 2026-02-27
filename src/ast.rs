//! Simplified AST.

use std::collections::BTreeMap;

use clang::{Entity, EntityKind, TranslationUnit};

use crate::ast::{
    c_enum::CEnum, c_function::CFunction, c_struct::CStruct, c_type::AliasType, c_var::CVar,
};

pub mod c_enum;
pub mod c_function;
pub mod c_struct;
pub mod c_type;
pub mod c_var;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Node<V> {
    pub name: String,
    pub comment: String,
    pub payload: V,
}

impl<V> Node<V> {
    pub fn from_entity<'a>(v: V, e: &'a Entity<'a>) -> Self {
        let name = e.get_name().unwrap_or_default();
        let comment = e.get_comment().unwrap_or_default();
        Self {
            name,
            comment,
            payload: v,
        }
    }
}

#[derive(Default, Debug)]
pub struct HeaderContent {
    pub global_variables: BTreeMap<String, Node<CVar>>,
    pub typedefs: BTreeMap<String, Node<AliasType>>,
    pub structs: BTreeMap<String, Node<CStruct>>,
    pub enums: BTreeMap<String, Node<CEnum>>,
    pub functions: BTreeMap<String, Node<CFunction>>,
}

impl HeaderContent {
    pub fn analyse(tu: TranslationUnit<'_>) -> anyhow::Result<Self> {
        let mut content = Self::default();
        for item in tu.get_entity().get_children() {
            if !item.is_in_main_file() {
                continue;
            }

            match item.get_kind() {
                EntityKind::VarDecl => {
                    let var = CVar::try_from_clang(item)?;
                    let node = Node::from_entity(var, &item);
                    content.global_variables.insert(node.name.clone(), node);
                }
                EntityKind::TypedefDecl => {
                    let alias = AliasType::try_from_clang(item)?;
                    let node = Node::from_entity(alias, &item);
                    content.typedefs.insert(node.name.clone(), node);
                }
                EntityKind::StructDecl => {
                    let s = CStruct::try_from_clang(item)?;
                    let node = Node::from_entity(s, &item);
                    content.structs.insert(node.name.clone(), node);
                }
                EntityKind::EnumDecl => {
                    let e = CEnum::try_from_clang(item)?;
                    let node = Node::from_entity(e, &item);
                    content.enums.insert(node.name.clone(), node);
                }
                EntityKind::FunctionDecl => {
                    let f = CFunction::try_from_clang(item)?;
                    let node = Node::from_entity(f, &item);
                    content.functions.insert(node.name.clone(), node);
                }
                _ => (),
            }
        }
        Ok(content)
    }

    pub fn symbols(&self) -> impl Iterator<Item = &String> {
        self.global_variables.keys().chain(self.functions.keys())
    }
}
