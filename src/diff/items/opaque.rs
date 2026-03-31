use crate::{
    ast::c_opaque::OpaqueDecl,
    diff::{Change, Compatibility},
};

pub struct OpaqueDiff {
    pub old: OpaqueDecl,
    pub new: OpaqueDecl,
}

impl Change for OpaqueDiff {
    fn compat(&self) -> Compatibility {
        // we don't know what this opaque type represents and what is behind it
        Compatibility::Dubious
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
