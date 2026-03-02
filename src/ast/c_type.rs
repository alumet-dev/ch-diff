use std::fmt::Display;

use anyhow::{Context, anyhow};
use clang::{Entity, EntityKind, Type, TypeKind};

use crate::ast::{
    Node,
    c_struct::{CStruct, StructField},
    c_union::CUnion,
};

#[derive(Debug, PartialEq, Clone)]
pub struct CType {
    display: String,
    pub kind: SimplifiedTypeKind,
}

impl Display for CType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display)
    }
}

/// A simplified enum for c types.
/// See https://en.cppreference.com/w/c/language/type.html.
#[derive(Debug, PartialEq, Clone)]
pub enum SimplifiedTypeKind {
    Basic(BasicType),
    Enum(EnumeratedType),
    Array(Box<ArrayType>),

    /// Structure or union.
    Record(NamedRecordType),

    /// Anonymous structure.
    AnonStruct(Box<CStruct>),
    AnonUnion(Box<CUnion>),

    /// Pointer type like `char*`.
    Pointer(Box<PointerType>),

    /// Typedef alias.
    Typedef(Box<AliasType>),

    /// A standard int type like `uint8_t` or `size_t`.
    StandardInt(StandardIntType),
}

#[derive(Debug, PartialEq, Clone)]
pub struct BasicType(pub TypeKind);
#[derive(Debug, PartialEq, Clone)]
pub struct EnumeratedType {
    pub enum_name: String,
    pub underlying_type: BasicType,
}
#[derive(Debug, PartialEq, Clone)]
pub struct ArrayType {
    pub element_type: CType,
    pub size: Option<usize>,
}
#[derive(Debug, PartialEq, Clone)]
pub struct NamedRecordType {
    pub name: String,
}
#[derive(Debug, PartialEq, Clone)]
pub struct PointerType {
    pub pointee: CType,
}
#[derive(Debug, PartialEq, Clone)]
pub struct AliasType {
    /// The name of the typedef alias.
    pub alias: String,
    /// The canonical underlying type.
    pub underlying: CType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StandardIntType {
    /// Fixed-width integers, signed, with its size in bits.
    IntFixed(u8),
    /// Fixed-width integers, unsigned, with its size in bits.
    /// For instance, `uint8_t` is `UIntFixed(8)`.
    UIntFixed(u8),
    IntPtr,
    UIntPtr,
    Size,
}

impl StandardIntType {
    pub fn is_signed(&self) -> bool {
        match self {
            StandardIntType::IntFixed(_) => true,
            StandardIntType::UIntFixed(_) => false,
            StandardIntType::IntPtr => true,
            StandardIntType::UIntPtr => false,
            StandardIntType::Size => false,
        }
    }
}

impl<'tu> TryFrom<Type<'tu>> for CType {
    type Error = anyhow::Error;

    fn try_from(t: Type<'tu>) -> Result<Self, Self::Error> {
        let kind: SimplifiedTypeKind = match t.get_kind() {
            TypeKind::Unexposed => return Err(anyhow!("unexposed type {t:?}")),
            TypeKind::ConstantArray => {
                let size = t.get_size();
                let element_type = t.get_element_type().unwrap();
                let element_type = CType::try_from(element_type)?;
                SimplifiedTypeKind::Array(Box::new(ArrayType { element_type, size }))
            }
            TypeKind::VariableArray | TypeKind::IncompleteArray => {
                let size = None;
                let element_type = t.get_element_type().unwrap();
                let element_type = CType::try_from(element_type)?;
                SimplifiedTypeKind::Array(Box::new(ArrayType { element_type, size }))
            }
            TypeKind::Record => {
                // is this an anonymous struct or union?
                if let Some(decl) = t.get_declaration()
                    && decl.is_anonymous()
                {
                    match decl.get_kind() {
                        EntityKind::StructDecl => {
                            let anon = CStruct::try_from_clang(decl).with_context(|| {
                                format!(
                                    "failed to parse anonymous structure {}",
                                    t.get_display_name()
                                )
                            })?;
                            SimplifiedTypeKind::AnonStruct(Box::new(anon))
                        }
                        EntityKind::UnionDecl => {
                            let anon = CUnion::try_from_clang(decl).with_context(|| {
                                format!("failed to parse anonymous union {}", t.get_display_name())
                            })?;
                            SimplifiedTypeKind::AnonUnion(Box::new(anon))
                        }
                        other => {
                            return Err(anyhow::Error::msg(format!(
                                "unexpected kind for an anonymous record: {other:?}"
                            )));
                        }
                    }
                } else {
                    let record = NamedRecordType {
                        name: t.get_display_name(),
                    };
                    SimplifiedTypeKind::Record(record)
                }
            }
            TypeKind::Pointer => {
                let pointee = t.get_pointee_type().unwrap().try_into()?;
                SimplifiedTypeKind::Pointer(Box::new(PointerType { pointee }))
            }
            TypeKind::Typedef => {
                let name = t.get_typedef_name().unwrap();
                let size = t.get_sizeof();
                match (name.as_str(), size) {
                    // unsigned fixed-width integers
                    ("uint64_t", Ok(8)) => {
                        SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(64))
                    }
                    ("uint32_t", Ok(4)) => {
                        SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(32))
                    }
                    ("uint16_t", Ok(2)) => {
                        SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(16))
                    }
                    ("uint8_t", Ok(1)) => {
                        SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(8))
                    }

                    // signed fixed-width integers
                    ("int64_t", Ok(8)) => {
                        SimplifiedTypeKind::StandardInt(StandardIntType::IntFixed(64))
                    }
                    ("int32_t", Ok(4)) => {
                        SimplifiedTypeKind::StandardInt(StandardIntType::IntFixed(32))
                    }
                    ("int16_t", Ok(2)) => {
                        SimplifiedTypeKind::StandardInt(StandardIntType::IntFixed(16))
                    }
                    ("int8_t", Ok(1)) => {
                        SimplifiedTypeKind::StandardInt(StandardIntType::IntFixed(8))
                    }

                    // other standard int types
                    ("uintptr_t", _) => SimplifiedTypeKind::StandardInt(StandardIntType::UIntPtr),
                    ("intptr_t", _) => SimplifiedTypeKind::StandardInt(StandardIntType::IntPtr),
                    ("size_t", _) => SimplifiedTypeKind::StandardInt(StandardIntType::Size),

                    // other typedef
                    _ => {
                        let underlying = t.get_canonical_type().try_into()?;
                        SimplifiedTypeKind::Typedef(Box::new(AliasType {
                            alias: name,
                            underlying,
                        }))
                    }
                }
            }
            TypeKind::Elaborated => {
                let underlying = t.get_elaborated_type().unwrap();
                return CType::try_from(underlying);
            }
            t => SimplifiedTypeKind::Basic(BasicType(t)),
        };

        let display = t.get_display_name();
        Ok(Self { display, kind })
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

#[derive(Debug, PartialEq, Clone, Copy, Eq, PartialOrd, Ord)]
pub enum CTypeComparison {
    Same,
    /// Ex: different typedefs that resolve to the same underlying type.
    Equivalent,
    Different,
}

impl CType {
    pub fn compare(&self, other: &CType) -> CTypeComparison {
        fn compare_fields<'a>(
            fields_a: impl Iterator<Item = &'a Node<StructField>>,
            fields_b: impl Iterator<Item = &'a Node<StructField>>,
        ) -> CTypeComparison {
            let mut res = CTypeComparison::Same;
            for (a, b) in fields_a.zip(fields_b) {
                res = res.max(a.payload.typ.compare(&b.payload.typ));
            }
            res
        }

        match (&self.kind, &other.kind) {
            (a, b) if a == b => CTypeComparison::Same,
            (SimplifiedTypeKind::Enum(a), SimplifiedTypeKind::Enum(b))
                if a.underlying_type == b.underlying_type =>
            {
                CTypeComparison::Equivalent
            }
            (SimplifiedTypeKind::Typedef(a), SimplifiedTypeKind::Typedef(b))
                if a.underlying == b.underlying =>
            {
                CTypeComparison::Equivalent
            }
            (SimplifiedTypeKind::AnonStruct(a), SimplifiedTypeKind::AnonStruct(b)) => {
                if a.fields.len() == b.fields.len() {
                    compare_fields(a.fields.values(), b.fields.values())
                } else {
                    CTypeComparison::Different
                }
            }
            (SimplifiedTypeKind::AnonUnion(a), SimplifiedTypeKind::AnonUnion(b)) => {
                if a.fields.len() == b.fields.len() {
                    compare_fields(a.fields.iter(), b.fields.iter())
                } else {
                    CTypeComparison::Different
                }
            }
            _ => CTypeComparison::Different,
        }
    }
}
