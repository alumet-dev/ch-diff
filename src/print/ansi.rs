use std::{
    fmt::Display,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use crate::{
    diff::{
        Change, ChangeBuf, ChangeKind, DeclChange, DiffReport, SourceDiff, enums::EnumChange,
        functions::FunctionChange, structs::StructChange, variables::VarChange,
    },
    print::types::{CLikeTypePrinter, RustLikeTypePrinter, TypePrinter, TypePrintingStyle},
};
use anyhow::Context;
use colored::{Color, ColoredString, Colorize, Styles};
use similar::{ChangeTag, TextDiff};

pub struct AnsiPrinter {
    writer: Box<dyn Write>,
    type_printer: Box<dyn TypePrinter>,
    options: AnsiOptions,
}

#[derive(Debug)]
pub struct AnsiOptions {
    pub type_style: TypePrintingStyle,
    pub print_diff_sign: bool,
}

impl AnsiPrinter {
    fn new(writer: impl Write + 'static, options: AnsiOptions) -> anyhow::Result<Self> {
        let type_printer: Box<dyn TypePrinter> = match options.type_style {
            TypePrintingStyle::C => Box::new(CLikeTypePrinter {}),
            TypePrintingStyle::Rust => Box::new(RustLikeTypePrinter {}),
        };
        Ok(Self {
            writer: Box::new(writer) as _,
            type_printer,
            options,
        })
    }

    pub fn to_file(output: File, options: AnsiOptions) -> anyhow::Result<Self> {
        Self::new(BufWriter::new(output), options)
    }

    pub fn create_file(output: &Path, options: AnsiOptions) -> anyhow::Result<Self> {
        let output = File::create(output)
            .with_context(|| format!("could not create new file {output:?}"))?;
        Self::to_file(output, options)
    }

    pub fn to_stdout(options: AnsiOptions) -> anyhow::Result<Self> {
        Self::new(BufWriter::new(std::io::stdout()), options)
    }
}

trait Printable {
    fn print_ansi(&self, p: &mut AnsiPrinter) -> anyhow::Result<()>;
}

impl super::ReportPrinter for AnsiPrinter {
    fn print_report(&mut self, report: DiffReport) -> anyhow::Result<()> {
        fn compat_color(compat: ChangeKind) -> Color {
            match compat {
                ChangeKind::BackwardCompatible => Color::Green,
                ChangeKind::Dubious => Color::Yellow,
                ChangeKind::Breaking => Color::Red,
            }
        }

        fn write_md_list<D: Display>(
            p: &mut AnsiPrinter,
            iter: impl IntoIterator<Item = D>,
            color: Color,
        ) -> anyhow::Result<()> {
            let iter = iter.into_iter();
            // print each element as a md list
            for elem in iter {
                writeln!(p.writer, "{}", format!("- {elem}").color(color))?;
            }
            write!(p.writer, "\n")?;
            Ok(())
        }

        fn write_detailed_changes<C: Change + Printable>(
            p: &mut AnsiPrinter,
            changes: &ChangeBuf<C>,
        ) -> anyhow::Result<()> {
            writeln!(p.writer, "\n#### Details\n")?;
            for change in changes {
                write!(p.writer, "- ")?;
                change.print_ansi(p)?;
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
            write_md_list(self, report.symbols.removed, Color::Red)?;
        }
        write!(self.writer, "Added symbols: ")?;
        if report.symbols.added.is_empty() {
            writeln!(self.writer, "∅")?;
        } else {
            write!(self.writer, "\n")?;
            write_md_list(self, report.symbols.added, Color::Green)?;
        }

        // global vars
        if !report.global_vars.is_empty() {
            writeln!(self.writer, "\n## Global Variables\n")?;
        }
        for (i, change) in report.global_vars.changes.into_iter().enumerate() {
            let n = i + 1;
            writeln!(self.writer, "### {n}. {}", change.var_name())?;
            change.print_ansi(self)?;
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
                    diff.source_diff.print_ansi(self)?;
                    write_detailed_changes(self, &diff.changes)?;
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
                    diff.source_diff.print_ansi(self)?;
                    write_detailed_changes(self, &diff.changes)?;
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
                    diff.source_diff.print_ansi(self)?;
                    write_detailed_changes(self, &diff.changes)?;

                    // After the details of each change, print the definition of anonymous members, if any.
                    if !diff.old_anon.is_empty() {
                        writeln!(self.writer, "\nwhere (old anonymous members):")?;
                        for (anon_id, anon_def) in diff.old_anon.iter() {
                            writeln!(self.writer, "- <anon{anon_id}>: {anon_def}")?;
                        }
                    }
                    if !diff.new_anon.is_empty() {
                        writeln!(self.writer, "\nwhere (new anonymous members):")?;
                        for (anon_id, anon_def) in diff.new_anon.iter() {
                            writeln!(self.writer, "- <anon{anon_id}>: {anon_def}")?;
                        }
                    }
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
                    diff.source_diff.print_ansi(self)?;
                    write_detailed_changes(self, &diff.changes)?;
                }
            }
        }
        Ok(())
    }
}

impl Printable for VarChange {
    fn print_ansi(&self, p: &mut AnsiPrinter) -> anyhow::Result<()> {
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
            VarChange::Added(node) => {
                writeln!(
                    p.writer,
                    "added variable: {}",
                    format!("+ {}", node.payload).green()
                )?;
            }
            VarChange::Removed(node) => {
                writeln!(
                    p.writer,
                    "removed variable: {}",
                    format!("- {}", node.payload).red()
                )?;
            }
        };
        Ok(())
    }
}

impl Printable for FunctionChange {
    fn print_ansi(&self, p: &mut AnsiPrinter) -> anyhow::Result<()> {
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
                    format!("{old_typ}").red(),
                    format!("{new_typ}").green()
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
                    "type of {name} changed: {} -> {}",
                    format!("{old_typ}").red(),
                    format!("{new_typ}").green()
                )?;
            }
            FunctionChange::ParameterRemoved(arg) => {
                writeln!(
                    p.writer,
                    "parameter removed: {}",
                    format!("{:?}", arg.name).red(),
                )?;
            }
            FunctionChange::ParameterAdded(arg) => {
                writeln!(
                    p.writer,
                    "parameter added: {}",
                    format!("{:?}", arg.name).red(),
                )?;
            }
        };
        Ok(())
    }
}

impl Printable for EnumChange {
    fn print_ansi(&self, p: &mut AnsiPrinter) -> anyhow::Result<()> {
        match self {
            EnumChange::ValueAdded(node) => {
                writeln!(
                    p.writer,
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
                    format!("{:?}", node.name).red(),
                )?;
            }
            EnumChange::TypeChanged { old, new } => {
                let old_typ = p.type_printer.type_to_string(old)?;
                let new_typ = p.type_printer.type_to_string(new)?;
                writeln!(
                    p.writer,
                    "enum type changed: {} -> {}",
                    format!("{old_typ}").red(),
                    format!("{new_typ}").green(),
                )?;
            }
        };
        Ok(())
    }
}

impl Printable for StructChange {
    fn print_ansi(&self, p: &mut AnsiPrinter) -> anyhow::Result<()> {
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
                    format!("{old_name}").red(),
                    format!("{new_name}").green(),
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
                write!(p.writer, "\n")?;
            }
            StructChange::FieldAdded(node) => {
                writeln!(p.writer, "field added: {}", node.name.green())?;
            }
            StructChange::FieldRemoved(node) => {
                writeln!(p.writer, "field removed: {}", node.name.red())?;
            }
            StructChange::FieldMoved {
                name,
                old_offset,
                new_offset,
            } => {
                write!(p.writer, "field moved: `{name}`: ")?;
                InlineDiff(old_offset, new_offset).print_ansi(p)?;
                write!(p.writer, "\n")?;
            }
        };
        Ok(())
    }
}

impl Printable for SourceDiff {
    fn print_ansi(&self, p: &mut AnsiPrinter) -> anyhow::Result<()> {
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
            }
        }
        write!(p.writer, "\n")?;
        Ok(())
    }
}

struct InlineDiff<D: Display + PartialEq>(D, D);

impl<D: Display + PartialEq> Printable for InlineDiff<D> {
    fn print_ansi(&self, p: &mut AnsiPrinter) -> anyhow::Result<()> {
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
