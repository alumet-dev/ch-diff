use std::{
    fmt::Display,
    fs::File,
    io::{BufWriter, Stdout, Write},
    path::Path,
};

use crate::diff::{
    Change, ChangeBuf, ChangeKind, DeclChange, DiffReport, SourceDiff, enums::EnumChange,
    functions::FunctionChange, structs::StructChange, variables::VarChange,
};
use anyhow::Context;
use colored::{Color, ColoredString, Colorize, Style, Styles};
use similar::{ChangeTag, DiffableStr, TextDiff};

pub struct AnsiPrinter<W: Write> {
    writer: W,
    // options: AnsiOptions, // TODO
}

// TODO
// #[derive(Debug)]
// pub struct AnsiOptions {
//     pub diff_sign: bool,
// }

impl AnsiPrinter<BufWriter<File>> {
    pub fn to_file(output: File) -> anyhow::Result<Self> {
        Ok(Self {
            writer: BufWriter::new(output),
        })
    }

    pub fn create_file(output: &Path) -> anyhow::Result<Self> {
        let output = File::create(output)
            .with_context(|| format!("could not create new file {output:?}"))?;
        Self::to_file(output)
    }
}

impl AnsiPrinter<BufWriter<Stdout>> {
    pub fn to_stdout() -> Self {
        Self {
            writer: BufWriter::new(std::io::stdout()),
        }
    }
}

trait Printable {
    fn print_ansi(&self, writer: &mut impl Write) -> anyhow::Result<()>;
}


impl<W: Write> super::ReportPrinter for AnsiPrinter<W> {
    fn print_report(&mut self, report: DiffReport) -> anyhow::Result<()> {
        fn compat_color(compat: ChangeKind) -> Color {
            match compat {
                ChangeKind::BackwardCompatible => Color::Green,
                ChangeKind::Dubious => Color::Yellow,
                ChangeKind::Breaking => Color::Red,
            }
        }

        fn write_md_list<D: Display>(
            writer: &mut impl Write,
            iter: impl IntoIterator<Item = D>,
            color: Color,
        ) -> anyhow::Result<()> {
            let iter = iter.into_iter();
            // print each element as a md list
            for elem in iter {
                writeln!(writer, "{}", format!("- {elem}").color(color))?;
            }
            write!(writer, "\n")?;
            Ok(())
        }

        fn write_detailed_changes<C: Change + Printable>(
            writer: &mut impl Write,
            changes: &ChangeBuf<C>,
        ) -> anyhow::Result<()> {
            writeln!(writer, "\n#### Details\n")?;
            for change in changes {
                write!(writer, "- ")?;
                change.print_ansi(writer)?;
            }
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
            write!(self.writer, "\n")?;
            write_md_list(&mut self.writer, report.symbols.removed, Color::Red)?;
        }
        write!(self.writer, "Added symbols: ")?;
        if report.symbols.added.is_empty() {
            writeln!(self.writer, "∅")?;
        } else {
            write!(self.writer, "\n")?;
            write_md_list(&mut self.writer, report.symbols.added, Color::Green)?;
        }

        // global vars
        if !report.global_vars.is_empty() {
            writeln!(self.writer, "\n## Global Variables\n")?;
        }
        for (i, change) in report.global_vars.changes.into_iter().enumerate() {
            let n = i + 1;
            writeln!(self.writer, "### {n}. {}", change.var_name())?;
            change.print_ansi(&mut self.writer)?;
        }

        // functions
        if !report.functions.is_empty() {
            writeln!(self.writer, "\n## Functions")?;
        }
        for (name, changes) in report.functions.into_iter() {
            let qual = "fn";
            match changes {
                DeclChange::Removed => {
                    writeln!(self.writer, "\n### {} {name}\n", qual.strikethrough())?;
                    writeln!(self.writer, "{}", "removed".red())?;
                }
                DeclChange::Changed(diff) => {
                    writeln!(self.writer, "\n### {qual} {name}\n")?;
                    diff.source_diff.print_ansi(&mut self.writer)?;
                    write_detailed_changes(&mut self.writer, &diff.changes)?;
                }
            }
        }

        // enums
        if !report.enums.is_empty() {
            writeln!(self.writer, "\n## Enums")?;
        }
        for (name, changes) in report.enums.into_iter() {
            let qual = "enum";
            match changes {
                DeclChange::Removed => {
                    writeln!(self.writer, "\n### {} {name}\n", qual.strikethrough())?;
                    writeln!(self.writer, "{}", "removed".red())?;
                }
                DeclChange::Changed(diff) => {
                    writeln!(self.writer, "\n### {qual} {name}\n")?;
                    diff.source_diff.print_ansi(&mut self.writer)?;
                    write_detailed_changes(&mut self.writer, &diff.changes)?;
                }
            }
        }

        // structs
        if !report.structs.is_empty() {
            writeln!(self.writer, "\n## Structures")?;
        }
        for (name, changes) in report.structs.into_iter() {
            let qual = "struct";
            match changes {
                DeclChange::Removed => {
                    writeln!(self.writer, "\n### {} {name}\n", qual.strikethrough())?;
                    writeln!(self.writer, "{}", "removed".red())?;
                }
                DeclChange::Changed(diff) => {
                    writeln!(self.writer, "\n### {qual} {name}\n")?;
                    diff.source_diff.print_ansi(&mut self.writer)?;
                    write_detailed_changes(&mut self.writer, &diff.changes)?;
                }
            }
        }

        // unions
        if !report.unions.is_empty() {
            writeln!(self.writer, "\n## Unions")?;
        }
        for (name, changes) in report.unions.into_iter() {
            let qual = "union";
            match changes {
                DeclChange::Removed => {
                    writeln!(self.writer, "\n### {} {name}\n", qual.strikethrough())?;
                    writeln!(self.writer, "{}", "removed".red())?;
                }
                DeclChange::Changed(diff) => {
                    writeln!(self.writer, "\n### {qual} {name}\n")?;
                    diff.source_diff.print_ansi(&mut self.writer)?;
                    write_detailed_changes(&mut self.writer, &diff.changes)?;
                }
            }
        }
        Ok(())
    }
}

impl Printable for VarChange {
    fn print_ansi(&self, writer: &mut impl Write) -> anyhow::Result<()> {
        match self {
            VarChange::TypeChanged {
                old_typ, new_typ, ..
            } => {
                writeln!(
                    writer,
                    "type changed:\n{}\n{}",
                    format!("- {}", old_typ.clang_display_name()).red(),
                    format!("+ {}", new_typ.clang_display_name()).green()
                )?;
            }
            VarChange::Added(node) => {
                writeln!(
                    writer,
                    "added variable: {}",
                    format!("+ {}", node.payload).green()
                )?;
            }
            VarChange::Removed(node) => {
                writeln!(
                    writer,
                    "removed variable: {}",
                    format!("- {}", node.payload).red()
                )?;
            }
        };
        Ok(())
    }
}

impl Printable for FunctionChange {
    fn print_ansi(&self, writer: &mut impl Write) -> anyhow::Result<()> {
        match self {
            FunctionChange::ParameterRenamed {
                old_name, new_name, ..
            } => {
                writeln!(
                    writer,
                    "parameter renamed: {} -> {}",
                    old_name.red(),
                    new_name.green()
                )?;
            }
            FunctionChange::ReturnTypeChanged { old_typ, new_typ } => {
                writeln!(
                    writer,
                    "return type changed: {} -> {}",
                    format!("{old_typ:?}").red(),
                    format!("{new_typ:?}").green()
                )?;
            }
            FunctionChange::ParameterTypeChanged {
                name,
                pos: _,
                old_typ,
                new_typ,
            } => {
                writeln!(
                    writer,
                    "type of {name} changed: {} -> {}",
                    format!("{old_typ:?}").red(),
                    format!("{new_typ:?}").green()
                )?;
            }
            FunctionChange::ParameterRemoved(arg) => {
                writeln!(
                    writer,
                    "parameter removed: {}",
                    format!("{:?}", arg.name).red(),
                )?;
            }
            FunctionChange::ParameterAdded(arg) => {
                writeln!(
                    writer,
                    "parameter added: {}",
                    format!("{:?}", arg.name).red(),
                )?;
            }
        };
        Ok(())
    }
}

impl Printable for EnumChange {
    fn print_ansi(&self, writer: &mut impl Write) -> anyhow::Result<()> {
        match self {
            EnumChange::ValueAdded(node) => {
                writeln!(
                    writer,
                    "value added: {}",
                    format!("{:?}", node.name).green(),
                )?;
            }
            EnumChange::ValueRenamed {
                old_name,
                new_name,
                value: _,
            } => {
                writeln!(
                    writer,
                    "value renamed: {} -> {}",
                    old_name.red(),
                    new_name.green()
                )?;
            }
            EnumChange::ValueRemoved(node) => {
                writeln!(
                    writer,
                    "value removed: {}",
                    format!("{:?}", node.name).red(),
                )?;
            }
            EnumChange::TypeChanged { old, new } => {
                writeln!(
                    writer,
                    "enum type changed: {} -> {}",
                    format!("{old:?}").red(),
                    format!("{new:?}").green(),
                )?;
            }
        };
        Ok(())
    }
}

impl Printable for StructChange {
    fn print_ansi(&self, writer: &mut impl Write) -> anyhow::Result<()> {
        match self {
            StructChange::SizeDiff { old_size, new_size } => {
                writeln!(
                    writer,
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
                    writer,
                    "field renamed: {} -> {}",
                    format!("{old_name}").red(),
                    format!("{new_name}").green(),
                )?;
            }
            StructChange::FieldChanged { name, old, new } => {
                writeln!(writer, "field changed: `{name}`:")?;

                // offset
                write!(writer, "\toffset: ")?;
                InlineDiff(old.offset, new.offset).print_ansi(writer)?;

                // bit field width
                let old_width = old
                    .bit_field_width
                    .map(|s| s.to_string())
                    .unwrap_or("none".to_owned());
                let new_width = old
                    .bit_field_width
                    .map(|s| s.to_string())
                    .unwrap_or("none".to_owned());
                write!(writer, "\n\tbit_field_width: ")?;
                InlineDiff(old_width, new_width).print_ansi(writer)?;

                // type
                write!(writer, "\n\ttype: ")?;
                InlineDiff(&old.typ.clang_display_name(), &new.typ.clang_display_name())
                    .print_ansi(writer)?;
                write!(writer, "\n")?;
            }
            StructChange::FieldAdded(node) => {
                writeln!(writer, "field added: {}", node.name.green())?;
            }
            StructChange::FieldRemoved(node) => {
                writeln!(writer, "field removed: {}", node.name.red())?;
            }
            StructChange::FieldMoved {
                name,
                old_offset,
                new_offset,
            } => {
                write!(writer, "field moved: `{name}`: ")?;
                InlineDiff(old_offset, new_offset).print_ansi(writer)?;
                write!(writer, "\n")?;
            }
        };
        Ok(())
    }
}

impl Printable for SourceDiff {
    fn print_ansi(&self, writer: &mut impl Write) -> anyhow::Result<()> {
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
                // write!(writer, "{sign}")?; // TODO configurable
                // todo line number?
                for (emphasized, value) in change.iter_strings_lossy() {
                    let mut s = ColoredString::from(value.as_ref());
                    if emphasized {
                        s.style |= style;
                        s = s.color(color.unwrap());
                        s = s.underline();
                        write!(writer, "{s}")?;
                    } else {
                        let s = if let Some(color) = color {
                            s.color(color)
                        } else {
                            s
                        };
                        write!(writer, "{s}")?;
                    }
                }
            }
        }
        write!(writer, "\n")?;
        Ok(())
    }
}

struct InlineDiff<D: Display + PartialEq>(D, D);

impl<D: Display + PartialEq> Printable for InlineDiff<D> {
    fn print_ansi(&self, writer: &mut impl Write) -> anyhow::Result<()> {
        let InlineDiff(old, new) = self;
        if old == new {
            write!(writer, "{old}")?;
        } else {
            let old_red = old.to_string().red();
            let new_green = new.to_string().green();
            write!(writer, "{old_red} -> {new_green}")?;
        }
        Ok(())
    }
}
