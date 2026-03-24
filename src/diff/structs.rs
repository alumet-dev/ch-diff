//! Structures diff.
//! - warning: field renamed, typedef change but equivalent underlying type
//! - breaking: field removed, field added, field change (same name, different type)

use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::{
    ast::{
        Node,
        c_struct::{CStruct, StructField},
        c_type::{CTypeComparison, anon::AnonContext},
    },
    diff::{Change, ChangeBuf, ChangeContainer, ChangeKind, SourceDiff},
};

use super::SourceDiffStyle;

pub struct StructDiff {
    pub changes: ChangeBuf<StructChange>,
    pub source_diff: SourceDiff,
    pub old_anon: AnonContext,
    pub new_anon: AnonContext,
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
    /// Same name, different offset
    FieldMoved {
        name: String,
        old_offset: usize,
        new_offset: usize,
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
            StructChange::FieldMoved { .. } => ChangeKind::Breaking,
            StructChange::FieldAdded(_) => ChangeKind::Breaking,
            StructChange::FieldRemoved(_) => ChangeKind::Breaking,
        }
    }
}

impl StructDiff {
    pub fn compute_diff(a: &CStruct, b: &CStruct) -> anyhow::Result<Option<Self>> {
        let mut changes = ChangeBuf::new();

        // TODO option to account for size change in types

        // breaking change: size difference
        if a.size != b.size {
            changes.push(StructChange::SizeDiff {
                old_size: a.size,
                new_size: b.size,
            });
        }

        // Since we compare fields by offset below, when a field is moved (for instance because the fields before it have changed), it will be detected as "removed" (old offset) and then "added" (new offest).
        // To catch these changes and mark the field as "moved" properly, we keep the "removed" fields here.
        let mut removed: FxHashMap<String, Node<StructField>> =
            FxHashMap::with_hasher(FxBuildHasher);

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
                            removed.insert(field_a.name.clone(), field_a.to_owned());
                            changes.push(StructChange::FieldAdded(field_b.to_owned()));
                        }
                    }
                }
                None => {
                    // this offset no longer exists, the field has been removed
                    removed.insert(field_a.name.clone(), field_a.to_owned());
                }
            }
        }

        // check new fields
        for (offset_b, field_b) in b.fields.iter() {
            if !a.fields.contains_key(offset_b) {
                if let Some(field_a) = removed.remove(&field_b.name)
                    && field_a.payload.typ == field_b.payload.typ
                    && field_a.payload.bit_field_width == field_b.payload.bit_field_width
                {
                    // the field has been moved to another offset
                    changes.push(StructChange::FieldMoved {
                        name: field_a.name,
                        old_offset: field_a.payload.offset,
                        new_offset: field_b.payload.offset,
                    });
                } else {
                    // the field has been completely removed
                    changes.push(StructChange::FieldAdded(field_b.to_owned()));
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
                    style: SourceDiffStyle::Multiline,
                },
                new_anon: a.anonymous.clone(),
                old_anon: b.anonymous.clone(),
            }))
        }
    }
}

impl ChangeContainer for StructDiff {
    fn overall_kind(&self) -> ChangeKind {
        self.changes.compatibility
    }
}
