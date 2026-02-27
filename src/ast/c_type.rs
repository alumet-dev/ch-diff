use anyhow::anyhow;
use clang::{Entity, Type, TypeKind};

/// A simplified enum for c types.
/// See https://en.cppreference.com/w/c/language/type.html.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CType {
    Basic(BasicType),
    Enum(EnumeratedType),
    Array(Box<ArrayType>),
    StructOrUnion(NamedRecordType),
    Pointer(Box<PointerType>),

    /// Typedef alias.
    Typedef(Box<AliasType>),
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BasicType(pub TypeKind);
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumeratedType {
    pub enum_name: String,
    pub underlying_type: BasicType,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ArrayType {
    pub element_type: CType,
    pub size: Option<usize>,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NamedRecordType {
    pub name: String,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PointerType {
    pub pointee: CType,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AliasType {
    /// The name of the typedef alias.
    pub alias: String,
    /// The canonical underlying type.
    pub underlying: CType,
}

impl<'tu> TryFrom<Type<'tu>> for CType {
    type Error = anyhow::Error;

    fn try_from(value: Type<'tu>) -> Result<Self, Self::Error> {
        match value.get_kind() {
            TypeKind::Unexposed => Err(anyhow!("unexposed type {value:?}")),
            TypeKind::ConstantArray => {
                let size = value.get_size();
                let element_type = value.get_element_type().unwrap();
                let element_type = CType::try_from(element_type)?;
                Ok(CType::Array(Box::new(ArrayType { element_type, size })))
            }
            TypeKind::VariableArray | TypeKind::IncompleteArray => {
                let size = None;
                let element_type = value.get_element_type().unwrap();
                let element_type = CType::try_from(element_type)?;
                Ok(CType::Array(Box::new(ArrayType { element_type, size })))
            }
            TypeKind::Record => {
                let record = NamedRecordType {
                    name: value.get_display_name(),
                };
                Ok(CType::StructOrUnion(record))
            }
            TypeKind::Pointer => {
                let pointee = value.get_pointee_type().unwrap().try_into()?;
                Ok(CType::Pointer(Box::new(PointerType { pointee })))
            }
            TypeKind::Typedef => {
                let name = value.get_typedef_name().unwrap();
                let underlying = value.get_canonical_type().try_into()?;
                Ok(CType::Typedef(Box::new(AliasType {
                    alias: name,
                    underlying,
                })))
            }
            t => Ok(CType::Basic(BasicType(t))),
        }
    }
}

impl AliasType {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        let alias = e.get_name().unwrap_or_default();
        let mut underlying = e.get_typedef_underlying_type().unwrap();
        if underlying.get_kind() == TypeKind::Elaborated {
            underlying = underlying.get_canonical_type();
        }
        let underlying = underlying.try_into()?;
        Ok(Self { alias, underlying })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CTypeComparison {
    Same,
    /// Ex: different typedefs that resolve to the same underlying type.
    Equivalent,
    Different,
}

impl CType {
    pub fn compare(&self, other: &CType) -> CTypeComparison {
        match (self, other) {
            (a, b) if a == b => CTypeComparison::Same,
            (CType::Enum(a), CType::Enum(b)) if a.underlying_type == b.underlying_type => {
                CTypeComparison::Equivalent
            }
            (CType::Typedef(a), CType::Typedef(b)) if a.underlying == b.underlying => {
                CTypeComparison::Equivalent
            }
            _ => CTypeComparison::Different,
        }
    }
}
