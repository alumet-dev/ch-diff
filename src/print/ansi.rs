use std::{
    fmt::Display,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use crate::{
    diff::{
        Change, Compatibility, DeclDiff, DeclKind, SemanticDiff, SourceDiff, SourceDiffStyle,
        buffer::ChangeBuf,
        items::{
            enums::EnumChange, functions::FunctionChange, opaque::OpaqueDiff,
            structs::StructChange, variables::VarChange,
        },
        report::{Diff, DiffReport},
    },
    print::types::{CLikeTypePrinter, RustLikeTypePrinter, TypePrinter, TypePrintingStyle},
};
use anyhow::Context;
use colored::{Color, ColoredString, Colorize, Styles};
use similar::{ChangeTag, TextDiff};

pub struct AnsiPrinter<W: Write> {
    writer: W,
    type_printer: Box<dyn TypePrinter>,
    options: AnsiOptions,
}

#[derive(Debug)]
pub struct AnsiOptions {
    pub type_style: TypePrintingStyle,
    pub print_diff_sign: bool,
}

impl<W: Write> AnsiPrinter<W> {
    fn new(writer: W, options: AnsiOptions) -> anyhow::Result<Self> {
        let type_printer: Box<dyn TypePrinter> = match options.type_style {
            TypePrintingStyle::C => Box::new(CLikeTypePrinter {}),
            TypePrintingStyle::Rust => Box::new(RustLikeTypePrinter {}),
        };
        Ok(Self {
            writer,
            type_printer,
            options,
        })
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl AnsiPrinter<BufWriter<File>> {
    pub fn to_file(output: File, options: AnsiOptions) -> anyhow::Result<Self> {
        Self::new(BufWriter::new(output), options)
    }

    pub fn create_file(output: &Path, options: AnsiOptions) -> anyhow::Result<Self> {
        let output = File::create(output)
            .with_context(|| format!("could not create new file {output:?}"))?;
        Self::to_file(output, options)
    }
}

impl AnsiPrinter<std::io::Stdout> {
    pub fn to_stdout(options: AnsiOptions) -> anyhow::Result<Self> {
        Self::new(std::io::stdout(), options)
    }
}

trait Printable {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()>;
}

impl<W: Write> super::ReportPrinter for AnsiPrinter<W> {
    fn print_report(&mut self, report: &DiffReport) -> anyhow::Result<()> {
        fn compat_color(compat: Compatibility) -> Color {
            match compat {
                Compatibility::BackwardCompatible => Color::Green,
                Compatibility::Dubious => Color::Yellow,
                Compatibility::Breaking => Color::Red,
            }
        }

        fn write_md_list<W: Write, D: Display>(
            p: &mut AnsiPrinter<W>,
            iter: impl IntoIterator<Item = D>,
            color: Color,
        ) -> anyhow::Result<()> {
            let iter = iter.into_iter();
            // print each element as a md list
            for elem in iter {
                writeln!(p.writer, "{}", format!("- {elem}").color(color))?;
            }
            writeln!(p.writer)?;
            Ok(())
        }

        writeln!(self.writer, "# Compatibility Report\n")?;
        writeln!(self.writer, "- old version: {}", report.old_name)?;
        writeln!(self.writer, "- new version: {}", report.new_name)?;

        // global compat
        let compat = report.global_compatibility();
        writeln!(
            self.writer,
            "{}",
            format!("- global compatibility: **{compat}**").color(compat_color(compat)),
        )?;

        // symbols diff
        writeln!(self.writer, "\n## Public Symbols\n")?;
        let compat = report.symbols.compatibility();
        writeln!(
            self.writer,
            "{}",
            format!("list of symbols: **{compat}**").color(compat_color(compat)),
        )?;
        write!(self.writer, "\nRemoved symbols: ")?;
        if report.symbols.removed.is_empty() {
            writeln!(self.writer, "∅")?;
        } else {
            writeln!(self.writer)?;
            write_md_list(self, &report.symbols.removed, Color::Red)?;
        }
        write!(self.writer, "Added symbols: ")?;
        if report.symbols.added.is_empty() {
            writeln!(self.writer, "∅")?;
        } else {
            writeln!(self.writer)?;
            write_md_list(self, &report.symbols.added, Color::Green)?;
        }

        // declarations diff
        let kinds = [
            (DeclKind::GlobalVar, "var", "Global Variables"),
            (DeclKind::Function, "fn", "Functions"),
            (DeclKind::Enum, "enum", "Enums"),
            (DeclKind::Struct, "struct", "Structs"),
            (DeclKind::Union, "union", "Unions"),
            (DeclKind::Opaque, "opaque", "Opaque Types"),
        ];
        for (kind, prefix, section_name) in kinds {
            if !report.declarations[kind].is_empty() {
                writeln!(self.writer, "\n## {section_name}\n")?;

                for (name, diff) in report.declarations[kind].iter() {
                    writeln!(self.writer, "### {prefix} {name}\n")?;
                    diff.print_ansi(self)?;
                }
            }
        }
        Ok(())
    }
}

impl Printable for Diff {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()> {
        writeln!(p.writer, "#### Source\n")?;
        self.source.print_ansi(p)?;

        writeln!(p.writer, "#### Details\n")?;
        match &self.semantic {
            SemanticDiff::Added => writeln!(p.writer, "added")?,
            SemanticDiff::Removed => writeln!(p.writer, "removed")?,
            SemanticDiff::Modified(diff) => diff.print_ansi(p)?,
        }

        Ok(())
    }
}

fn write_change_list<W: Write, C: Change + Printable>(
    p: &mut AnsiPrinter<W>,
    changes: &ChangeBuf<C>,
) -> anyhow::Result<()> {
    for change in changes {
        writeln!(p.writer, "- ")?;
        change.print_ansi(p)?;
    }
    Ok(())
}

impl Printable for DeclDiff {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()> {
        match self {
            DeclDiff::GlobalVar(diff) => diff.print_ansi(p),
            DeclDiff::Enum(diff) => write_change_list(p, &diff.changes),
            DeclDiff::Struct(diff) => write_change_list(p, &diff.changes),
            DeclDiff::Union(diff) => write_change_list(p, &diff.changes),
            DeclDiff::Function(diff) => write_change_list(p, &diff.changes),
            DeclDiff::Opaque(diff) => diff.print_ansi(p),
        }
    }
}

impl Printable for OpaqueDiff {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()> {
        writeln!(
            p.writer,
            "kind changed: {} -> {}",
            self.old.kind.to_string().red(),
            self.new.kind.to_string().green()
        )?;
        Ok(())
    }
}

impl Printable for VarChange {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()> {
        match self {
            VarChange::TypeChanged {
                old_typ, new_typ, ..
            } => {
                let old_typ = p.type_printer.type_to_string(old_typ)?;
                let new_typ = p.type_printer.type_to_string(new_typ)?;
                writeln!(
                    p.writer,
                    "type changed:\n{}\n{}",
                    format!("- {}", old_typ).red(),
                    format!("+ {}", new_typ).green()
                )?;
            }
        };
        Ok(())
    }
}

impl Printable for FunctionChange {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()> {
        match self {
            FunctionChange::ParameterRenamed {
                old_name, new_name, ..
            } => {
                writeln!(
                    p.writer,
                    "parameter renamed: {} -> {}",
                    old_name.red(),
                    new_name.green()
                )?;
            }
            FunctionChange::ReturnTypeChanged { old_typ, new_typ } => {
                let old_typ = p.type_printer.type_to_string(old_typ)?;
                let new_typ = p.type_printer.type_to_string(new_typ)?;
                writeln!(
                    p.writer,
                    "return type changed: {} -> {}",
                    old_typ.red(),
                    new_typ.green()
                )?;
            }
            FunctionChange::ParameterTypeChanged {
                name,
                pos: _,
                old_typ,
                new_typ,
            } => {
                let old_typ = p.type_printer.type_to_string(old_typ)?;
                let new_typ = p.type_printer.type_to_string(new_typ)?;
                writeln!(
                    p.writer,
                    "type of `{name}` changed: {} -> {}",
                    old_typ.red(),
                    new_typ.green()
                )?;
            }
            FunctionChange::ParameterRemoved(arg) => {
                writeln!(p.writer, "parameter removed: {}", arg.name.red(),)?;
            }
            FunctionChange::ParameterAdded(arg) => {
                writeln!(p.writer, "parameter added: {}", arg.name.red(),)?;
            }
        };
        Ok(())
    }
}

impl Printable for EnumChange {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()> {
        match self {
            EnumChange::ValueAdded(node) => {
                writeln!(p.writer, "value added: {}", node.meta.name.green(),)?;
            }
            EnumChange::ValueRenamed {
                old_name,
                new_name,
                value: _,
            } => {
                writeln!(
                    p.writer,
                    "value renamed: {} -> {}",
                    old_name.red(),
                    new_name.green()
                )?;
            }
            EnumChange::ValueRemoved(node) => {
                writeln!(
                    p.writer,
                    "value removed: {}",
                    format!("{}", node.meta.name).red(),
                )?;
            }
            EnumChange::TypeChanged { old, new } => {
                let old_typ = p.type_printer.type_to_string(old)?;
                let new_typ = p.type_printer.type_to_string(new)?;
                writeln!(
                    p.writer,
                    "enum type changed: {} -> {}",
                    old_typ.red(),
                    new_typ.green(),
                )?;
            }
        };
        Ok(())
    }
}

impl Printable for StructChange {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()> {
        match self {
            StructChange::SizeDiff { old_size, new_size } => {
                writeln!(
                    p.writer,
                    "struct size changed: {} -> {} bytes",
                    format!("{old_size}").red(),
                    format!("{new_size}").green(),
                )?;
            }
            StructChange::FieldRenamed {
                old_name,
                new_name,
                field: _,
            } => {
                writeln!(
                    p.writer,
                    "field renamed: {} -> {}",
                    old_name.red(),
                    new_name.green(),
                )?;
            }
            StructChange::FieldChanged { name, old, new } => {
                writeln!(p.writer, "field changed: `{name}`:")?;

                // offset
                write!(p.writer, "\toffset: ")?;
                InlineDiff(old.offset, new.offset).print_ansi(p)?;

                // bit field width
                let old_width = old
                    .bit_field_width
                    .map(|s| s.to_string())
                    .unwrap_or("none".to_owned());
                let new_width = old
                    .bit_field_width
                    .map(|s| s.to_string())
                    .unwrap_or("none".to_owned());
                write!(p.writer, "\n\tbit_field_width: ")?;
                InlineDiff(old_width, new_width).print_ansi(p)?;

                // type
                write!(p.writer, "\n\ttype: ")?;
                let old_typ = p.type_printer.type_to_string(&old.typ)?;
                let new_typ = p.type_printer.type_to_string(&new.typ)?;
                InlineDiff(old_typ, new_typ).print_ansi(p)?;
                writeln!(p.writer)?;
            }
            StructChange::FieldAdded(node) => {
                writeln!(p.writer, "field added: {}", node.meta.name.green())?;
            }
            StructChange::FieldRemoved(node) => {
                writeln!(p.writer, "field removed: {}", node.meta.name.red())?;
            }
            StructChange::FieldMoved {
                name,
                old_offset,
                new_offset,
            } => {
                write!(p.writer, "field moved: `{name}`: ")?;
                InlineDiff(old_offset, new_offset).print_ansi(p)?;
                writeln!(p.writer)?;
            }
        };
        Ok(())
    }
}

impl Printable for SourceDiff {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()> {
        // compute diff
        let diff = TextDiff::from_lines(&self.old, &self.new);

        // todo option to hide some unchanged lines
        for op in diff.ops() {
            for change in diff.iter_inline_changes(op) {
                let (sign, style, color) = match change.tag() {
                    ChangeTag::Delete => ("-".red(), Styles::Clear, Some(Color::Red)),
                    ChangeTag::Insert => ("+".green(), Styles::Clear, Some(Color::Green)),
                    ChangeTag::Equal => (" ".into(), Styles::Dimmed, None),
                };
                if p.options.print_diff_sign {
                    write!(p.writer, "{sign}")?;
                }
                // todo line number?
                for (emphasized, value) in change.iter_strings_lossy() {
                    let mut s = ColoredString::from(value.as_ref());
                    if emphasized {
                        s.style |= style;
                        s = s.color(color.unwrap());
                        s = s.underline();
                        write!(p.writer, "{s}")?;
                    } else {
                        let s = if let Some(color) = color {
                            s.color(color)
                        } else {
                            s
                        };
                        write!(p.writer, "{s}")?;
                    }
                }
                if self.style == SourceDiffStyle::Split1v1 {
                    writeln!(p.writer)?;
                }
            }
        }
        writeln!(p.writer)?;
        Ok(())
    }
}

struct InlineDiff<D: Display + PartialEq>(D, D);

impl<D: Display + PartialEq> Printable for InlineDiff<D> {
    fn print_ansi<W: Write>(&self, p: &mut AnsiPrinter<W>) -> anyhow::Result<()> {
        let InlineDiff(old, new) = self;
        if old == new {
            write!(p.writer, "{old}")?;
        } else {
            let old_red = old.to_string().red();
            let new_green = new.to_string().green();
            write!(p.writer, "{old_red} -> {new_green}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    pub fn print_to_string(options: AnsiOptions, object: impl Printable) -> anyhow::Result<String> {
        let mut buf: Vec<u8> = Vec::new();
        let mut printer = AnsiPrinter::new(buf, options).context("printer construction error")?;
        object.print_ansi(&mut printer).context("printing error")?;
        buf = printer.into_inner();
        let str =
            String::from_utf8(buf).context("the printer produced invalid utf-8 characters")?;
        Ok(str)
    }

    #[test]
    fn print_source_diff_multiline() {
        let diff = SourceDiff {
            old: indoc! {"construct {
                a: u8
                b: u16
                c: what
            }"}
            .to_string(),
            new: indoc! {"construct {
                a: u8
                c: watt-hour
                d: zzz
            }"}
            .to_string(),
            style: SourceDiffStyle::Multiline,
        };
        let options = AnsiOptions {
            type_style: TypePrintingStyle::Rust,
            print_diff_sign: false,
        };
        let printed = print_to_string(options, diff).unwrap();
        println!("{}", printed);
    }

    #[test]
    fn print_source_diff_inline() {
        let diff = SourceDiff {
            old: indoc! {"fn f(a: u8, b: u16, c: what) -> R"}.to_string(),
            new: indoc! {"fn f(a: u8, c: watt-hour, d: zzz) -> R"}.to_string(),
            style: SourceDiffStyle::Split1v1,
        };
        let options = AnsiOptions {
            type_style: TypePrintingStyle::Rust,
            print_diff_sign: false,
        };
        let printed = print_to_string(options, diff).unwrap();
        println!("{}", printed);
    }
}
