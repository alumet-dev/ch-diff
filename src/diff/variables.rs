use crate::{
    ast::{
        HeaderContent, Node,
        c_type::{CType, CTypeComparison},
        c_var::CVar,
    },
    diff::{Change, ChangeBuf, ChangeContainer, ChangeKind},
};

pub struct GlobalVarDiff {
    pub changes: ChangeBuf<VarChange>,
}

#[derive(Debug, Clone)]
pub enum VarChange {
    TypeChanged {
        name: String,
        old_typ: CType,
        new_typ: CType,
    },
    Added(Node<CVar>),
    Removed(Node<CVar>),
}

impl VarChange {
    pub fn var_name(&self) -> &str {
        match self {
            VarChange::TypeChanged { name, .. } => &name,
            VarChange::Added(node) => &node.name,
            VarChange::Removed(node) => &node.name,
        }
    }
}

impl Change for VarChange {
    fn kind(&self) -> super::ChangeKind {
        match self {
            VarChange::TypeChanged {
                old_typ, new_typ, ..
            } => {
                if old_typ.compare(new_typ) == CTypeComparison::Equivalent {
                    ChangeKind::Dubious
                } else {
                    ChangeKind::Breaking
                }
            }
            VarChange::Added(_) => ChangeKind::BackwardCompatible,
            VarChange::Removed(_) => ChangeKind::Breaking,
        }
    }
}

impl GlobalVarDiff {
    pub fn compute_diff(a: &HeaderContent, b: &HeaderContent) -> anyhow::Result<Self> {
        let mut changes = ChangeBuf::new();

        // check existing variables
        for (name, var_a) in a.global_variables.iter() {
            match b.global_variables.get(name) {
                Some(var_b) => {
                    if var_a.payload.typ != var_b.payload.typ {
                        changes.push(VarChange::TypeChanged {
                            name: name.to_owned(),
                            old_typ: var_a.payload.typ.to_owned(),
                            new_typ: var_b.payload.typ.to_owned(),
                        });
                    }
                }
                None => changes.push(VarChange::Removed(var_a.clone())),
            }
        }

        // detect new variables
        for (name, var_b) in b.global_variables.iter() {
            if !a.global_variables.contains_key(name) {
                changes.push(VarChange::Added(var_b.to_owned()));
            }
        }

        Ok(Self { changes })
    }

    pub fn len(&self) -> usize {
        self.changes.changes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.changes.is_empty()
    }

    pub fn compatibility(&self) -> ChangeKind {
        self.changes.compatibility
    }
}

impl ChangeContainer for GlobalVarDiff {
    fn overall_kind(&self) -> ChangeKind {
        self.changes.compatibility
    }
}
