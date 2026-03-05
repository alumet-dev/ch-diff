//! Utilities for printing C types.

use std::io::Write;

use itertools::{Itertools, Position};

use crate::ast::c_type::{CType, SimplifiedTypeKind};

trait TypePrinter {
    fn print_type(&mut self, t: &CType) -> anyhow::Result<()>;
}

pub struct RustLikeTypePrinter<W: Write> {
    where_declarations: Vec<(String, CType)>,
    writer: W,
}

impl<W: Write> RustLikeTypePrinter<W> {
    fn rec_print_type(&mut self, t: &CType) -> anyhow::Result<()> {
        match &t.kind {
            // SimplifiedTypeKind::Basic(basic_type) => todo!(),
            // SimplifiedTypeKind::Enum(enumerated_type) => todo!(),
            SimplifiedTypeKind::Array(ty) => {
                // [elem ; n]
                write!(self.writer, "[")?;
                self.rec_print_type(&ty.element_type)?;
                write!(self.writer, " ; ")?;
                match ty.size {
                    Some(n) => write!(self.writer, "{n}")?,
                    None => write!(self.writer, "variable-length")?,
                }
                write!(self.writer, "]")?;
            }
            SimplifiedTypeKind::Record(named_record_type) => {
                write!(self.writer, "{}", &named_record_type.name)?
            }
            SimplifiedTypeKind::AnonStruct(anon) => {
                // the definition will be written at the end, in a "where" clause
            }
            SimplifiedTypeKind::AnonUnion(anon) => {
                todo!()
            }
            // SimplifiedTypeKind::Pointer(pointer_type) => todo!(),
            // SimplifiedTypeKind::Typedef(alias_type) => todo!(),
            // SimplifiedTypeKind::StandardInt(standard_int_type) => todo!(),
            _ => (),
        };
        Ok(())
    }
}

impl<W: Write> TypePrinter for RustLikeTypePrinter<W> {
    fn print_type(&mut self, t: &CType) -> anyhow::Result<()> {
        self.rec_print_type(t)?;
        if !self.where_declarations.is_empty() {
            write!(self.writer, "\n where ");
            for (pos, (key, typ)) in self.where_declarations.iter().with_position() {
                write!(self.writer, "{key}")?;
                // self.rec_print_type(typ)?; //TODO anon types can add new where decls…
                if pos != Position::Last && pos != Position::Only {
                    write!(self.writer, ",\n");
                }
            }
        }
        Ok(())
    }
}
