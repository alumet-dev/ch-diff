use core::fmt;
use std::fmt::Display;

use anyhow::{Context, anyhow};
use clang::{Entity, EntityKind, Type, TypeKind};

use crate::ast::{
    Node,
    c_struct::{CStruct, StructField},
    c_type::{
        anon::{AnonContext, AnonymousMemberId},
        stdint::StandardIntType,
    },
    c_union::CUnion,
};

#[derive(Debug, PartialEq, Clone)]
pub struct CType {
    pub kind: SimplifiedTypeKind,
    clang_display_name: String,
}

/// A simplified enum for c types.
/// See https://en.cppreference.com/w/c/language/type.html.
#[derive(Debug, PartialEq, Clone)]
pub enum SimplifiedTypeKind {
    OtherBasic(BasicType),
    Enum(EnumeratedType),
    Array(Box<ArrayType>),

    /// Structure or union.
    Record(NamedRecordType),

    /// Anonymous structure or union.
    ///
    /// To simplify the type analysis and printing, we store its definition elsewhere, in a [`AnonContext`](anon::AnonContext).
    Anonymous(AnonymousMemberId),

    /// Pointer type like `char*`.
    Pointer(Box<PointerType>),

    /// Typedef alias.
    Typedef(Box<AliasType>),

    /// A standard int type like `uint8_t` or `size_t`.
    StandardInt(StandardIntType),
}

pub mod anon {
    use derive_more::Display;

    use super::{CStruct, CUnion};

    #[derive(Debug, PartialEq, Clone)]
    pub enum AnonymousMemberDef {
        Struct(CStruct),
        Union(CUnion),
    }

    #[derive(Debug, PartialEq, Clone, Display)]
    #[display("{_0}")]
    pub struct AnonymousMemberId(usize);

    /// Registry of anonymous types (structs or unions).
    /// See https://en.cppreference.com/w/c/language/struct.html.
    #[derive(Debug, PartialEq, Clone)]
    pub struct AnonContext {
        anon_defs: Vec<AnonymousMemberDef>,
    }

    impl AnonContext {
        pub fn new() -> Self {
            Self {
                anon_defs: Vec::new(),
            }
        }
        pub fn register_struct(&mut self, anon_struct: CStruct) -> AnonymousMemberId {
            let i = self.anon_defs.len();
            self.anon_defs.push(AnonymousMemberDef::Struct(anon_struct));
            AnonymousMemberId(i)
        }

        pub fn register_union(&mut self, anon_union: CUnion) -> AnonymousMemberId {
            let i = self.anon_defs.len();
            self.anon_defs.push(AnonymousMemberDef::Union(anon_union));
            AnonymousMemberId(i)
        }

        pub fn get(&self, id: AnonymousMemberId) -> Option<&AnonymousMemberDef> {
            self.anon_defs.get(id.0)
        }

        pub fn iter(&self) -> impl Iterator<Item = (AnonymousMemberId, &AnonymousMemberDef)> {
            self.anon_defs
                .iter()
                .enumerate()
                .map(|(i, def)| (AnonymousMemberId(i), def))
        }

        pub fn is_empty(&self) -> bool {
            self.anon_defs.is_empty()
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    pub struct AnonStructId(usize);
    #[derive(Debug, PartialEq, Clone)]
    pub struct AnonUnionId(usize);
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

pub mod stdint {
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
}

impl CType {
    pub fn new(kind: SimplifiedTypeKind) -> Self {
        Self {
            kind,
            clang_display_name: String::from("?"),
        }
    }

    pub fn try_from_clang<'tu>(
        t: Type<'tu>,
        anon_ctx: Option<&mut AnonContext>,
    ) -> Result<Self, anyhow::Error> {
        let kind: SimplifiedTypeKind = match t.get_kind() {
            TypeKind::Unexposed => return Err(anyhow!("unexposed type {t:?}")),
            TypeKind::ConstantArray => {
                let size = t.get_size();
                let element_type = t.get_element_type().unwrap();
                let element_type = CType::try_from_clang(element_type, anon_ctx)?;
                SimplifiedTypeKind::Array(Box::new(ArrayType { element_type, size }))
            }
            TypeKind::VariableArray | TypeKind::IncompleteArray => {
                let size = None;
                let element_type = t.get_element_type().unwrap();
                let element_type = CType::try_from_clang(element_type, anon_ctx)?;
                SimplifiedTypeKind::Array(Box::new(ArrayType { element_type, size }))
            }
            TypeKind::Record => {
                // is this an anonymous struct or union?
                if let Some(decl) = t.get_declaration()
                    && decl.is_anonymous()
                {
                    let anon_ctx = anon_ctx.with_context(|| format!("unexpected anonymous member {} at this point (it should be in a struct or union, but anon_ctx is None here)", t.get_display_name()))?;
                    match decl.get_kind() {
                        EntityKind::StructDecl => {
                            let anon = CStruct::try_from_clang(decl).with_context(|| {
                                format!("failed to parse anonymous struct {}", t.get_display_name())
                            })?;
                            let anon_id = anon_ctx.register_struct(anon);
                            SimplifiedTypeKind::Anonymous(anon_id)
                        }
                        EntityKind::UnionDecl => {
                            let anon = CUnion::try_from_clang(decl).with_context(|| {
                                format!("failed to parse anonymous union {}", t.get_display_name())
                            })?;
                            let anon_id = anon_ctx.register_union(anon);
                            SimplifiedTypeKind::Anonymous(anon_id)
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
                let pointee = t.get_pointee_type().unwrap();
                let pointee = CType::try_from_clang(pointee, anon_ctx).with_context(|| {
                    format!(
                        "failed to convert pointee type `{pointee:?}` for pointer type {}",
                        t.get_display_name()
                    )
                })?;
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
                        let canonical = t.get_canonical_type();
                        let underlying =
                            CType::try_from_clang(canonical, anon_ctx).with_context(|| {
                                format!(
                                    "failed to convert canonical type `{}` of {}",
                                    canonical.get_display_name(),
                                    t.get_display_name()
                                )
                            })?;
                        SimplifiedTypeKind::Typedef(Box::new(AliasType {
                            alias: name,
                            underlying,
                        }))
                    }
                }
            }
            TypeKind::Elaborated => {
                let underlying = t.get_elaborated_type().unwrap();
                return CType::try_from_clang(underlying, anon_ctx).with_context(|| {
                    format!(
                        "failed to convert elaborated type `{}` of {}",
                        underlying.get_display_name(),
                        t.get_display_name()
                    )
                });
            }
            t => SimplifiedTypeKind::OtherBasic(BasicType(t)),
        };

        let display_name = t.get_display_name();
        Ok(Self {
            clang_display_name: display_name,
            kind,
        })
    }

    pub fn clang_display_name(&self) -> &str {
        &self.clang_display_name
    }
}

impl AliasType {
    pub fn try_from_clang<'a>(e: Entity<'a>) -> anyhow::Result<Self> {
        let alias = e.get_name().unwrap_or_default();
        let mut underlying = e.get_typedef_underlying_type().unwrap();
        if underlying.get_kind() == TypeKind::Elaborated {
            underlying = underlying.get_canonical_type();
        }
        let underlying = CType::try_from_clang(underlying, None).with_context(|| {
            format!(
                "failed to convert typedef underlying type {} for {e:?}",
                underlying.get_display_name(),
            )
        })?;
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
            (a, b) if a == b => CTypeComparison::Same,
            _ => CTypeComparison::Different,
        }
    }
}
