use crate::{
    ast::{
        c_type::{CType, CTypeComparison},
        c_var::CVar,
    },
    diff::{Change, Compatibility},
};

#[derive(Debug, Clone)]
pub enum VarChange {
    TypeChanged { old_typ: CType, new_typ: CType },
}

impl VarChange {
    pub fn compute_diff(a: &CVar, b: &CVar) -> anyhow::Result<Option<Self>> {
        if a.typ != b.typ {
            Ok(Some(Self::TypeChanged {
                old_typ: a.typ.to_owned(),
                new_typ: b.typ.to_owned(),
            }))
        } else {
            Ok(None)
        }
    }
}

impl Change for VarChange {
    fn compat(&self) -> Compatibility {
        match self {
            VarChange::TypeChanged { old_typ, new_typ } => {
                if old_typ.compare(new_typ) == CTypeComparison::Equivalent {
                    Compatibility::Dubious
                } else {
                    Compatibility::Breaking
                }
            }
        }
    }
}
