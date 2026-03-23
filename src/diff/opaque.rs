use crate::{ast::c_opaque::OpaqueDecl, diff::ChangeContainer};

pub struct OpaqueDiff {
    pub old: OpaqueDecl,
    pub new: OpaqueDecl,
}

impl ChangeContainer for OpaqueDiff {
    fn overall_kind(&self) -> super::ChangeKind {
        super::ChangeKind::Dubious
    }
}

impl OpaqueDiff {
    pub fn compute_diff(old: &OpaqueDecl, new: &OpaqueDecl) -> anyhow::Result<Option<OpaqueDiff>> {
        if old == new {
            return Ok(None);
        }
        Ok(Some(OpaqueDiff {
            old: old.to_owned(),
            new: new.to_owned(),
        }))
    }
}
