//! Simplified AST.

use std::{collections::BTreeMap, fmt::Display, path::PathBuf};

use anyhow::Context;
use clang::{Clang, Entity, EntityKind, TranslationUnit};

use crate::ast::{
    c_enum::CEnum, c_function::CFunction, c_opaque::OpaqueDecl, c_struct::CStruct,
    c_type::AliasType, c_union::CUnion, c_var::CVar,
};

pub mod c_enum;
pub mod c_function;
pub mod c_opaque;
pub mod c_struct;
pub mod c_type;
pub mod c_union;
pub mod c_var;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Node<V> {
    pub meta: NodeMetadata,
    pub payload: V,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NodeMetadata {
    pub name: String,
    pub comment: String,
    pub source_code: String,
}

impl NodeMetadata {
    pub fn from_entity<'a>(e: &'a Entity<'a>) -> Self {
        let name = e.get_name().unwrap_or_default();
        let comment = e.get_comment().map(|c| c.trim_ascii().to_owned()).unwrap_or_default();
        let source_code = e.get_pretty_printer().print();
        Self {
            name,
            comment,
            source_code,
        }
    }
}

impl<V> Node<V> {
    pub fn from_entity<'a>(v: V, e: &'a Entity<'a>) -> Self {
        let meta = NodeMetadata::from_entity(e);
        Self { meta, payload: v }
    }
}

impl<V> Display for Node<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.meta.source_code)
    }
}

#[derive(Debug)]
pub struct Header {
    pub file: PathBuf,
    pub content: HeaderContent,
}

#[derive(Default, Debug)]
pub struct HeaderContent {
    pub global_variables: BTreeMap<String, Node<CVar>>,
    pub typedefs: BTreeMap<String, Node<AliasType>>,
    pub structs: BTreeMap<String, Node<CStruct>>,
    pub unions: BTreeMap<String, Node<CUnion>>,
    pub enums: BTreeMap<String, Node<CEnum>>,
    pub functions: BTreeMap<String, Node<CFunction>>,
    pub opaques: BTreeMap<String, Node<OpaqueDecl>>,
}

impl Header {
    pub fn parse(clang: &Clang, file: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let file = file.into();
        let index = clang::Index::new(clang, true, true);
        let tu = index
            .parser(&file)
            .parse()
            .with_context(|| format!("failed to parse {file:?}"))?;
        let content =
            HeaderContent::analyse(tu).with_context(|| format!("failed to analyse {file:?}"))?;
        Ok(Self { file, content })
    }
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
                    content
                        .global_variables
                        .insert(node.meta.name.clone(), node);
                }
                EntityKind::TypedefDecl => {
                    let alias = AliasType::try_from_clang(item)?;
                    let node = Node::from_entity(alias, &item);
                    content.typedefs.insert(node.meta.name.clone(), node);
                }
                EntityKind::StructDecl | EntityKind::EnumDecl | EntityKind::UnionDecl
                    if !item.is_definition() =>
                {
                    // opaque type
                    let opaque = OpaqueDecl::try_from_clang(item)?;
                    let node = Node::from_entity(opaque, &item);
                    content.opaques.insert(node.meta.name.clone(), node);
                }
                EntityKind::StructDecl => {
                    let s = CStruct::try_from_clang(item)?;
                    let node = Node::from_entity(s, &item);
                    content.structs.insert(node.meta.name.clone(), node);
                }
                EntityKind::UnionDecl => {
                    let s = CUnion::try_from_clang(item)?;
                    let node = Node::from_entity(s, &item);
                    content.unions.insert(node.meta.name.clone(), node);
                }
                EntityKind::EnumDecl => {
                    let e = CEnum::try_from_clang(item)?;
                    let node = Node::from_entity(e, &item);
                    content.enums.insert(node.meta.name.clone(), node);
                }
                EntityKind::FunctionDecl => {
                    let f = CFunction::try_from_clang(item)?;
                    let node = Node::from_entity(f, &item);
                    content.functions.insert(node.meta.name.clone(), node);
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
