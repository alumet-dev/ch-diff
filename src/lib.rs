pub mod ast;
pub mod diff;
pub mod generate;
pub mod hist;
pub mod print;

#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum, derive_more::Display)]
#[display(rename_all = "lowercase")]
pub enum PathOutputStyle {
    Relative,
    Absolute,
}
