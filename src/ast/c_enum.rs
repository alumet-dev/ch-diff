use std::collections::BTreeMap;

use clang::{Entity, EntityKind, TypeKind};
use indexmap::IndexMap;

use crate::ast::c_type::{BasicType, CType, SimplifiedTypeKind};

#[derive(Debug, Clone)]
pub struct CEnum {
    pub underlying_type: CType,
    pub variants: BTreeMap<Value, super::Node<CEnumValue>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CEnumValue {
    pub value: Value,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Value {
    Signed(i64),
    Unsigned(u64),
}

impl CEnum {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        let underlying_type: CType = e
            .get_enum_underlying_type()
            .unwrap()
            .get_canonical_type()
            .try_into()?;
        let variants = e
            .get_children()
            .iter()
            .filter_map(|item| {
                if item.get_kind() != EntityKind::EnumConstantDecl {
                    return None;
                }

                let (v_signed, v_unsigned) = item.get_enum_constant_value().unwrap();
                let value = match &underlying_type.kind {
                    SimplifiedTypeKind::Basic(BasicType(
                        TypeKind::UShort | TypeKind::UInt | TypeKind::ULong,
                    )) => Value::Unsigned(v_unsigned),
                    SimplifiedTypeKind::Basic(BasicType(
                        TypeKind::Short | TypeKind::Int | TypeKind::Long,
                    )) => Value::Signed(v_signed),
                    SimplifiedTypeKind::StandardInt(i) => {
                        if i.is_signed() {
                            Value::Signed(v_signed)
                        } else {
                            Value::Unsigned(v_unsigned)
                        }
                    }
                    ty => panic!("unexpected type for enum value: {ty:?}"),
                };
                let variant = CEnumValue { value };
                let variant = super::Node::from_entity(variant, item);
                Some((variant.payload.value, variant))
            })
            .collect();

        Ok(Self {
            underlying_type,
            variants,
        })
    }
}
