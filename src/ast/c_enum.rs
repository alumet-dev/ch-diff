use std::{collections::BTreeMap, fmt::Display};

use clang::{Entity, EntityKind, TypeKind};

use crate::ast::c_type::{BasicType, CType, SimplifiedTypeKind};

#[derive(Debug, Clone)]
pub struct CEnum {
    pub underlying_type: CType,
    pub variants: BTreeMap<Value, super::Node<CEnumValue>>,
    display: String,
}

impl Display for CEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display)
    }
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
        let underlying_type = e.get_enum_underlying_type().unwrap().get_canonical_type();
        let underlying_type = CType::try_from_clang(underlying_type, None)?;
        let variants = e
            .get_children()
            .iter()
            .filter_map(|item| {
                if item.get_kind() != EntityKind::EnumConstantDecl {
                    return None;
                }

                let (v_signed, v_unsigned) = item.get_enum_constant_value().unwrap();
                let value = match &underlying_type.kind {
                    SimplifiedTypeKind::OtherBasic(BasicType(
                        TypeKind::UShort | TypeKind::UInt | TypeKind::ULong | TypeKind::CharU,
                    )) => Value::Unsigned(v_unsigned),
                    SimplifiedTypeKind::OtherBasic(BasicType(
                        TypeKind::Short | TypeKind::Int | TypeKind::Long | TypeKind::CharS,
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

        let display = e.get_pretty_printer().print();
        Ok(Self {
            underlying_type,
            variants,
            display,
        })
    }
}
