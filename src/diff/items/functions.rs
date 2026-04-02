//! Functions diff.
//! - warning: parameter renamed, typedef change but equivalent underlying type
//! - breaking: return type change, parameter removed, parameter added, parameter change (same name, different type)

use itertools::{EitherOrBoth, Itertools};

use crate::{
    ast::{
        c_function::{CFunction, FunctionArg},
        c_type::{CType, CTypeComparison},
    },
    diff::{Change, Compatibility, buffer::ChangeBuf},
};

pub struct FunctionDiff {
    pub changes: ChangeBuf<FunctionChange>,
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
    fn compat(&self) -> Compatibility {
        match self {
            FunctionChange::ParameterRenamed { .. } => Compatibility::Dubious,
            FunctionChange::ReturnTypeChanged { old_typ, new_typ } => {
                if old_typ.compare(new_typ) == CTypeComparison::Equivalent {
                    Compatibility::Dubious
                } else {
                    Compatibility::Breaking
                }
            }
            FunctionChange::ParameterTypeChanged {
                old_typ, new_typ, ..
            } => {
                if old_typ.compare(new_typ) == CTypeComparison::Equivalent {
                    Compatibility::Dubious
                } else {
                    Compatibility::Breaking
                }
            }
            FunctionChange::ParameterRemoved(_) => Compatibility::Breaking,
            FunctionChange::ParameterAdded(_) => Compatibility::Breaking,
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
            Ok(Some(Self { changes }))
        }
    }
}

impl Change for FunctionDiff {
    fn compat(&self) -> Compatibility {
        self.changes.compatibility()
    }
}
