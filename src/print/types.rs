//! Utilities for printing C types.

use std::{borrow::Cow, fmt::Display, fmt::Write};

use anyhow::Context;
use itertools::Itertools;

use crate::ast::c_type::{CType, SimplifiedTypeKind, stdint::StandardIntType};

/// Trait for printing types.
pub trait TypePrinter {
    /// Prints a type to any writer.
    fn print_type(&mut self, writer: &mut dyn Write, t: &CType) -> anyhow::Result<()>;

    /// Prints a type to a new string.
    fn type_to_string(&mut self, t: &CType) -> anyhow::Result<String> {
        let mut str = String::new();
        self.print_type(&mut str, t)
            .with_context(|| format!("printing failure for {t:?}"))?;
        Ok(str)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, clap::ValueEnum)]
pub enum TypePrintingStyle {
    C,
    Rust,
}

// TODO we'll add options later
pub struct CLikeTypePrinter {}
pub struct RustLikeTypePrinter {}

impl RustLikeTypePrinter {
    fn rec_print_type<W: Write + ?Sized>(
        &mut self,
        writer: &mut W,
        t: &CType,
    ) -> anyhow::Result<()> {
        match &t.kind {
            SimplifiedTypeKind::Array(ty) => {
                // [elem ; n]
                write!(writer, "[")?;
                self.rec_print_type(writer, &ty.element_type)?;
                write!(writer, "; ")?;
                match ty.size {
                    Some(n) => write!(writer, "{n}")?,
                    None => write!(writer, "variable-length")?,
                }
                write!(writer, "]")?;
            }
            SimplifiedTypeKind::Record(ty) => write!(writer, "{}", &ty.name)?,
            SimplifiedTypeKind::Enum(ty) => write!(writer, "{}", ty.enum_name)?,
            SimplifiedTypeKind::Typedef(ty) => {
                write!(writer, "{}", ty.alias)?;
            }
            SimplifiedTypeKind::Anonymous(id) => {
                // the definition of anon types is meant to be printed somewhere else
                write!(writer, "<anon{id}>")?;
            }
            SimplifiedTypeKind::Pointer(ty) => {
                // *pointee
                write!(writer, "*")?;
                self.rec_print_type(writer, &ty.pointee)?;
            }
            SimplifiedTypeKind::StandardInt(ty) => match ty {
                StandardIntType::IntFixed(bits) => write!(writer, "i{bits}")?,
                StandardIntType::UIntFixed(bits) => write!(writer, "u{bits}")?,
                StandardIntType::IntPtr => write!(writer, "intptr_t")?,
                StandardIntType::UIntPtr => write!(writer, "uintptr_t")?,
                StandardIntType::Size => write!(writer, "size_t")?,
            },
            SimplifiedTypeKind::OtherBasic(ty) => write!(writer, "{:?}", ty.0)?,
        };
        Ok(())
    }
}

impl TypePrinter for RustLikeTypePrinter {
    fn print_type(&mut self, writer: &mut dyn Write, t: &CType) -> anyhow::Result<()> {
        self.rec_print_type(writer, t)
    }
}

// C types are weird
struct CDecl {
    basic_type: String,
    chain: Vec<ChainItem>,
}

struct CDeclString {
    left: Vec<Cow<'static, str>>,
    right_rev: Vec<Cow<'static, str>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ChainItem {
    Array(Option<usize>),
    Pointer,
}

impl CDecl {
    fn build(t: &CType) -> anyhow::Result<Self> {
        let mut decl = Self {
            basic_type: String::new(),
            chain: Vec::new(),
        };
        decl.rec_build(t)?;
        Ok(decl)
    }

    fn rec_build(&mut self, t: &CType) -> anyhow::Result<()> {
        match &t.kind {
            SimplifiedTypeKind::Array(ty) => {
                self.chain.push(ChainItem::Array(ty.size));
                self.rec_build(&ty.element_type)?;
            }
            SimplifiedTypeKind::Pointer(ty) => {
                self.chain.push(ChainItem::Pointer);
                self.rec_build(&ty.pointee)?;
            }
            SimplifiedTypeKind::Record(ty) => {
                self.basic_type = ty.name.to_owned();
            }
            SimplifiedTypeKind::Enum(ty) => {
                self.basic_type = ty.enum_name.to_owned();
            }
            SimplifiedTypeKind::Typedef(ty) => {
                self.basic_type = ty.alias.to_owned();
            }
            SimplifiedTypeKind::Anonymous(id) => {
                self.basic_type = format!("<anon{id}>");
            }
            SimplifiedTypeKind::StandardInt(ty) => {
                let name = match ty {
                    StandardIntType::IntFixed(bits) => format!("int{bits}_t"),
                    StandardIntType::UIntFixed(bits) => format!("uint{bits}_t"),
                    StandardIntType::IntPtr => "intptr_t".to_owned(),
                    StandardIntType::UIntPtr => "uintptr_t".to_owned(),
                    StandardIntType::Size => "size_t".to_owned(),
                };
                self.basic_type = name;
            }
            SimplifiedTypeKind::OtherBasic(ty) => {
                self.basic_type = format!("{ty:?}").to_lowercase();
            }
        };
        Ok(())
    }

    fn into_string(self) -> String {
        assert!(!self.basic_type.is_empty());
        let mut str = CDeclString::with_basic_type(self.basic_type);
        log::trace!("chain: {:?}", self.chain);

        match self.chain.as_slice() {
            [] => (),
            [single] => str.push(single, None),
            _ => {
                for (item, parent) in self.chain.iter().rev().tuple_windows() {
                    log::trace!("item: {item:?}, parent: {parent:?}");
                    str.push(item, Some(parent));
                }
                // handle the last item (last in rev = first)
                let last = self.chain.first().unwrap();
                str.push(last, None);
            }
        }

        str.finish();
        str.to_string()
    }
}

impl CDeclString {
    fn with_basic_type(basic: String) -> Self {
        Self {
            left: vec![basic.into(), " ".into()],
            right_rev: Vec::new(),
        }
    }

    fn push(&mut self, item: &ChainItem, parent: Option<&ChainItem>) {
        match item {
            ChainItem::Array(size) => {
                match size {
                    Some(n) => self.right_rev.push(format!("[{n}]").into()),
                    None => self.right_rev.push("[]".into()),
                };
                match parent {
                    Some(ChainItem::Pointer) => {
                        // we must add parens, for example in (*var)[2]
                        self.left.push("(".into());
                        self.right_rev.push(")".into());
                    }
                    _ => (),
                }
            }
            ChainItem::Pointer => {
                self.left.push("*".into());
            }
        }
    }

    fn finish(&mut self) {
        // remove extra space
        if self.left.len() == 2 {
            self.left.truncate(1);
        }
    }
}

impl Display for CDeclString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        log::trace!("left: {:?}", self.left);
        log::trace!("right: {:?}", self.right_rev);
        for elem in self.left.iter() {
            f.write_str(elem.as_ref())?;
        }
        for elem in self.right_rev.iter().rev() {
            f.write_str(elem.as_ref())?;
        }
        Ok(())
    }
}

impl TypePrinter for CLikeTypePrinter {
    fn print_type(&mut self, writer: &mut dyn Write, t: &CType) -> anyhow::Result<()> {
        let decl = CDecl::build(t)?;
        let str = decl.into_string();
        write!(writer, "{str}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::c_type::{ArrayType, PointerType};

    use super::*;
    use pretty_assertions::assert_eq;

    fn test_print_type_c(expected: &str, ty: &SimplifiedTypeKind) {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Trace)
            .try_init();
        let t = CType::new(ty.to_owned());
        let mut printer = CLikeTypePrinter {};
        let res = printer.type_to_string(&t).expect("printing failed");
        assert_eq!(expected, res);
    }

    fn test_print_type_rust(expected: &str, ty: &SimplifiedTypeKind) {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Trace)
            .try_init();
        let t = CType::new(ty.to_owned());
        let mut printer = RustLikeTypePrinter {};
        let res = printer.type_to_string(&t).expect("printing failed");
        assert_eq!(expected, res);
    }

    #[test]
    fn test_basic_type() {
        let ty = SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(8));
        test_print_type_c("uint8_t", &ty);
        test_print_type_rust("u8", &ty);
    }

    #[test]
    fn test_pointer() {
        let ty = SimplifiedTypeKind::Pointer(Box::new(PointerType {
            pointee: CType::new(SimplifiedTypeKind::StandardInt(StandardIntType::IntFixed(
                16,
            ))),
        }));
        test_print_type_c("int16_t *", &ty);
        test_print_type_rust("*i16", &ty);
    }

    #[test]
    fn test_array() {
        let ty = SimplifiedTypeKind::Array(Box::new(ArrayType {
            element_type: CType::new(SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(
                32,
            ))),
            size: Some(5),
        }));
        test_print_type_c("uint32_t[5]", &ty);
        test_print_type_rust("[u32; 5]", &ty);
    }

    #[test]
    fn test_arrays() {
        // array 3 of (array 4 of (array 5 of uint32_t))
        let uint32 = CType::new(SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(
            32,
        )));
        let array5 = SimplifiedTypeKind::Array(Box::new(ArrayType {
            element_type: uint32,
            size: Some(5),
        }));
        let array4 = SimplifiedTypeKind::Array(Box::new(ArrayType {
            element_type: CType::new(array5),
            size: Some(4),
        }));
        let array3 = SimplifiedTypeKind::Array(Box::new(ArrayType {
            element_type: CType::new(array4),
            size: Some(3),
        }));
        test_print_type_c("uint32_t[3][4][5]", &array3);
        test_print_type_rust("[[[u32; 5]; 4]; 3]", &array3);
    }

    #[test]
    fn test_pointer_to_array() {
        let array = SimplifiedTypeKind::Array(Box::new(ArrayType {
            element_type: CType::new(SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(
                32,
            ))),
            size: Some(5),
        }));
        let ty = SimplifiedTypeKind::Pointer(Box::new(PointerType {
            pointee: CType::new(array),
        }));
        test_print_type_c("uint32_t (*)[5]", &ty);
        test_print_type_rust("*[u32; 5]", &ty);
    }

    #[test]
    fn test_array_of_pointers() {
        let ptr = SimplifiedTypeKind::Pointer(Box::new(PointerType {
            pointee: CType::new(SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(
                8,
            ))),
        }));
        let ty = SimplifiedTypeKind::Array(Box::new(ArrayType {
            element_type: CType::new(ptr),
            size: Some(10),
        }));
        test_print_type_c("uint8_t *[10]", &ty);
        test_print_type_rust("[*u8; 10]", &ty);
    }

    #[test]
    fn test_very_complex_combination() {
        let ptr_to_u8 = SimplifiedTypeKind::Pointer(Box::new(PointerType {
            pointee: CType::new(SimplifiedTypeKind::StandardInt(StandardIntType::UIntFixed(
                8,
            ))),
        }));

        let array_of_ptr_to_u8 = SimplifiedTypeKind::Array(Box::new(ArrayType {
            element_type: CType::new(ptr_to_u8),
            size: Some(2),
        }));

        let ptr_to_array_of_ptr_to_u8 = SimplifiedTypeKind::Pointer(Box::new(PointerType {
            pointee: CType::new(array_of_ptr_to_u8),
        }));

        let ty = SimplifiedTypeKind::Array(Box::new(ArrayType {
            element_type: CType::new(ptr_to_array_of_ptr_to_u8),
            size: Some(3),
        }));

        test_print_type_c("uint8_t *(*[3])[2]", &ty);
        test_print_type_rust("[*[*u8; 2]; 3]", &ty);
    }
}
