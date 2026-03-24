//! Functions diff.
//! - warning: parameter renamed, typedef change but equivalent underlying type
//! - breaking: return type change, parameter removed, parameter added, parameter change (same name, different type)

use itertools::{EitherOrBoth, Itertools};

use crate::{
    ast::{
        c_function::{CFunction, FunctionArg},
        c_type::{CType, CTypeComparison},
    },
    diff::{Change, ChangeBuf, ChangeContainer, ChangeKind, SourceDiff, SourceDiffStyle},
};

pub struct FunctionDiff {
    pub changes: ChangeBuf<FunctionChange>,
    pub source_diff: SourceDiff,
}

#[derive(Debug)]
pub enum FunctionChange {
    ParameterRenamed {
        old_name: String,
        new_name: String,
        param: FunctionArg,
    },

    ReturnTypeChanged {
        old_typ: CType,
        new_typ: CType,
    },

    ParameterTypeChanged {
        name: String,
        pos: usize,
        old_typ: CType,
        new_typ: CType,
    },

    ParameterRemoved(FunctionArg),
    ParameterAdded(FunctionArg),
}

impl Change for FunctionChange {
    fn kind(&self) -> super::ChangeKind {
        match self {
            FunctionChange::ParameterRenamed { .. } => ChangeKind::Dubious,
            FunctionChange::ReturnTypeChanged { old_typ, new_typ } => {
                if old_typ.compare(new_typ) == CTypeComparison::Equivalent {
                    ChangeKind::Dubious
                } else {
                    ChangeKind::Breaking
                }
            }
            FunctionChange::ParameterTypeChanged {
                old_typ, new_typ, ..
            } => {
                if old_typ.compare(new_typ) == CTypeComparison::Equivalent {
                    ChangeKind::Dubious
                } else {
                    ChangeKind::Breaking
                }
            }
            FunctionChange::ParameterRemoved(_) => ChangeKind::Breaking,
            FunctionChange::ParameterAdded(_) => ChangeKind::Breaking,
        }
    }
}

impl FunctionDiff {
    pub fn compute_diff(a: &CFunction, b: &CFunction) -> anyhow::Result<Option<Self>> {
        let mut changes = ChangeBuf::new();

        // return type
        if a.return_type != b.return_type {
            changes.push(FunctionChange::ReturnTypeChanged {
                old_typ: a.return_type.clone(),
                new_typ: b.return_type.clone(),
            });
        }

        // compare args by position
        for item in a.arguments.iter().zip_longest(b.arguments.iter()) {
            match item {
                EitherOrBoth::Both(arg_a, arg_b) => {
                    match (arg_a.name == arg_b.name, arg_a.typ == arg_b.typ) {
                        (true, true) => {
                            // no change
                        }
                        (true, false) => {
                            // same pos, same name, different type
                            changes.push(FunctionChange::ParameterTypeChanged {
                                name: arg_a.name.clone(),
                                pos: arg_a.pos,
                                old_typ: arg_a.typ.clone(),
                                new_typ: arg_b.typ.clone(),
                            });
                        }
                        (false, true) => {
                            // same pos, different name, same type
                            changes.push(FunctionChange::ParameterRenamed {
                                old_name: arg_a.name.clone(),
                                new_name: arg_b.name.clone(),
                                param: arg_a.to_owned(),
                            });
                        }
                        (false, false) => {
                            // same pos, different name, different type => let's say that the old arg is gone and a new one has been put here
                            changes.push(FunctionChange::ParameterRemoved(arg_a.to_owned()));
                            changes.push(FunctionChange::ParameterAdded(arg_b.to_owned()));
                        }
                    }
                }
                EitherOrBoth::Left(removed_arg) => {
                    // the new version has fewer args than the old version
                    changes.push(FunctionChange::ParameterRemoved(removed_arg.to_owned()));
                }
                EitherOrBoth::Right(new_arg) => {
                    // the new version has *more* args
                    changes.push(FunctionChange::ParameterAdded(new_arg.to_owned()));
                }
            }
        }

        if changes.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Self {
                changes,
                source_diff: SourceDiff {
                    old: a.to_string(),
                    new: b.to_string(),
                    style: SourceDiffStyle::Split1v1,
                },
            }))
        }
    }
}

impl ChangeContainer for FunctionDiff {
    fn overall_kind(&self) -> ChangeKind {
        self.changes.compatibility
    }
}
