//! Enums diff.
//! - ok: new values added
//! - warning: values renamed (same value but different name)
//! - breaking: values removed (underlying integer value no longer present in enum), values changed (same name, different integer value)

use crate::{
    ast::{
        Node,
        c_enum::{CEnum, CEnumValue},
        c_type::CType,
    },
    diff::{Change, ChangeBuf, ChangeKind},
};

pub struct EnumDiff {
    pub changes: ChangeBuf<EnumChange>,
}

pub enum EnumChange {
    /// New enum value added. This is backward-compatible.
    ValueAdded(Node<CEnumValue>),

    ValueRenamed {
        old_name: String,
        new_name: String,
        value: CEnumValue,
    },

    ValueRemoved(Node<CEnumValue>),

    /// The underlying type of the enum has changed.
    TypeChanged {
        old: CType,
        new: CType,
    },
}

impl Change for EnumChange {
    fn kind(&self) -> super::ChangeKind {
        match self {
            EnumChange::ValueAdded(_) => ChangeKind::BackwardCompatible,
            EnumChange::ValueRenamed { .. } => ChangeKind::Dubious,
            EnumChange::ValueRemoved(_) => ChangeKind::Breaking,
            EnumChange::TypeChanged { .. } => ChangeKind::Breaking,
        }
    }
}

impl EnumDiff {
    pub fn compute_diff(a: &CEnum, b: &CEnum) -> anyhow::Result<Self> {
        let mut changes = ChangeBuf::new();

        // check type
        if a.underlying_type != b.underlying_type {
            changes.push(EnumChange::TypeChanged {
                old: a.underlying_type.clone(),
                new: b.underlying_type.clone(),
            });
        }

        // check existing values
        for (value_a, v_a) in a.variants.iter() {
            match b.variants.get(value_a) {
                Some(v_b) => {
                    // compare the two variants
                    if v_a.name != v_b.name {
                        changes.push(EnumChange::ValueRenamed {
                            old_name: v_a.name.clone(),
                            new_name: v_b.name.clone(),
                            value: v_a.payload.clone(),
                        });
                    }
                }
                None => {
                    // variant removed
                    changes.push(EnumChange::ValueRemoved(v_a.to_owned()));
                }
            }
        }

        // check new values
        for (value_b, v_b) in b.variants.iter() {
            if !a.variants.contains_key(value_b) {
                changes.push(EnumChange::ValueAdded(v_b.to_owned()));
            }
        }

        Ok(Self { changes })
    }
}
