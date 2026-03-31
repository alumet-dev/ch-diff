//! Structures diff.
//! - warning: field renamed, typedef change but equivalent underlying type
//! - breaking: field removed, field added, field change (same name, different type)

use itertools::{EitherOrBoth, Itertools};

use crate::{
    ast::c_union::CUnion,
    diff::{Change, ChangeBuf, Compatibility, items::structs::StructChange},
};

pub struct UnionDiff {
    pub changes: ChangeBuf<StructChange>,
}

impl UnionDiff {
    pub fn compute_diff(a: &CUnion, b: &CUnion) -> anyhow::Result<Option<Self>> {
        let mut changes = ChangeBuf::new();

        // breaking change: size difference
        if a.size != b.size {
            changes.push(StructChange::SizeDiff {
                old_size: a.size,
                new_size: b.size,
            });
        }

        // Compare the different possibilities (1 union field = 1 possibility, but there may be equivalent types…).
        // TODO improve this
        for either in a.fields.iter().zip_longest(b.fields.iter()) {
            match either {
                EitherOrBoth::Both(a, b) => {
                    match (a.meta.name == b.meta.name, a.payload.typ == b.payload.typ) {
                        (true, true) => {
                            // no change
                        }
                        (true, false) => {
                            // same pos, same name, different type
                            changes.push(StructChange::FieldChanged {
                                name: a.meta.name.clone(),
                                old: a.payload.clone(),
                                new: b.payload.clone(),
                            });
                        }
                        (false, true) => {
                            // same pos, different name, same type
                            changes.push(StructChange::FieldRenamed {
                                old_name: a.meta.name.clone(),
                                new_name: b.meta.name.clone(),
                                field: a.payload.clone(),
                            });
                        }
                        (false, false) => {
                            // same pos, different name, different type => let's say that the old arg is gone and a new one has been put here
                            changes.push(StructChange::FieldRemoved(a.to_owned()));
                            changes.push(StructChange::FieldAdded(b.to_owned()));
                        }
                    }
                }
                EitherOrBoth::Left(removed_field) => {
                    // the new version has fewer possibilities than the old version
                    changes.push(StructChange::FieldRemoved(removed_field.to_owned()));
                }
                EitherOrBoth::Right(added_field) => {
                    // the new version has *more* possibilities
                    changes.push(StructChange::FieldAdded(added_field.to_owned()));
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

impl Change for UnionDiff {
    fn compat(&self) -> Compatibility {
        self.changes.compatibility
    }
}
