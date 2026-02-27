//! Structures diff.
//! - warning: field renamed, typedef change but equivalent underlying type
//! - breaking: field removed, field added, field change (same name, different type)

use crate::{
    ast::{
        Node,
        c_struct::{CStruct, StructField},
        c_type::CTypeComparison,
    },
    diff::{Change, ChangeBuf, ChangeKind},
};

pub struct StructDiff {
    pub changes: ChangeBuf<StructChange>,
}

#[derive(Debug)]
pub enum StructChange {
    /// Structure size changed.
    SizeDiff {
        old_size: usize,
        new_size: usize,
    },
    /// Same offset and type, different name.
    FieldRenamed {
        old_name: String,
        new_name: String,
        field: StructField,
    },
    /// Same offset and name, different type
    FieldChanged {
        name: String,
        old: StructField,
        new: StructField,
    },
    FieldAdded(Node<StructField>),
    FieldRemoved(Node<StructField>),
}

impl Change for StructChange {
    fn kind(&self) -> ChangeKind {
        match self {
            StructChange::SizeDiff { .. } => ChangeKind::Breaking,
            StructChange::FieldRenamed { .. } => ChangeKind::Dubious,
            StructChange::FieldChanged { old, new, .. } => {
                if old.typ.compare(&new.typ) == CTypeComparison::Equivalent {
                    ChangeKind::Dubious
                } else {
                    ChangeKind::Breaking
                }
            }
            StructChange::FieldAdded(_) => ChangeKind::Breaking,
            StructChange::FieldRemoved(_) => ChangeKind::Breaking,
        }
    }
}

impl StructDiff {
    pub fn compute_diff(a: &CStruct, b: &CStruct) -> anyhow::Result<Self> {
        let mut changes = ChangeBuf::new();

        // breaking change: size difference
        if a.size != b.size {
            changes.push(StructChange::SizeDiff {
                old_size: a.size,
                new_size: b.size,
            });
        }

        // compare the fields, by offset
        for (offset_a, field_a) in a.fields.iter() {
            match b.fields.get(offset_a) {
                Some(field_b) => {
                    // compare field_a and field_b
                    let typ_a = (&field_a.payload.typ, &field_a.payload.bit_field_width);
                    let typ_b = (&field_b.payload.typ, &field_b.payload.bit_field_width);
                    let name_a = &field_a.name;
                    let name_b = &field_b.name;

                    match (typ_a == typ_b, name_a == name_b) {
                        (true, true) => {
                            // no change
                            continue;
                        }
                        (true, false) => {
                            // same type, different name
                            changes.push(StructChange::FieldRenamed {
                                old_name: name_a.clone(),
                                new_name: name_b.clone(),
                                field: field_a.payload.clone(),
                            });
                        }
                        (false, true) => {
                            // different type, same name
                            changes.push(StructChange::FieldChanged {
                                name: name_a.clone(),
                                old: field_a.payload.clone(),
                                new: field_b.payload.clone(),
                            });
                        }
                        (false, false) => {
                            // different type and name, they are probably completely unrelated
                            changes.push(StructChange::FieldRemoved(field_a.to_owned()));
                            changes.push(StructChange::FieldAdded(field_b.to_owned()));
                        }
                    }
                }
                None => {
                    // this offset no longer exists, the field has been removed
                    changes.push(StructChange::FieldRemoved(field_a.to_owned()));
                }
            }
        }

        // check new fields
        for (offset_b, field_b) in b.fields.iter() {
            if !a.fields.contains_key(offset_b) {
                changes.push(StructChange::FieldAdded(field_b.to_owned()));
            }
        }

        Ok(Self { changes })
    }
}
