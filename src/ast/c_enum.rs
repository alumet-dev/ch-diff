use clang::{Entity, EntityKind, TypeKind};
use indexmap::IndexMap;

#[derive(Debug)]
pub struct CEnum {
    pub underlying_type: TypeKind,
    pub variants: IndexMap<String, super::Node<CEnumValue>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CEnumValue {
    pub value: Value,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Signed(i64),
    Unsigned(u64),
}

impl CEnum {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        let underlying_type = e.get_enum_underlying_type().unwrap().get_kind(); // todo canonical?
        let variants = e
            .get_children()
            .iter()
            .filter_map(|item| {
                if item.get_kind() != EntityKind::EnumConstantDecl {
                    return None;
                }

                let value = item.get_enum_constant_value().unwrap();
                let value = match underlying_type {
                    TypeKind::UShort | TypeKind::UInt | TypeKind::ULong => Value::Unsigned(value.1),
                    TypeKind::Short | TypeKind::Int | TypeKind::Long => Value::Signed(value.0),
                    ty => panic!("unexpected type for enum value: {ty:?}"),
                };
                let variant = CEnumValue { value };
                let variant = super::Node::from_entity(variant, item);
                Some((variant.name.clone(), variant))
            })
            .collect();

        Ok(Self {
            underlying_type,
            variants,
        })
    }
}
