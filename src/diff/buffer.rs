use crate::diff::{Change, Compatibility};

#[derive(Debug, Clone)]
pub struct ChangeBuf<C: Change> {
    changes: Vec<C>,
    compatibility: Compatibility,
}

impl<C: Change> ChangeBuf<C> {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            compatibility: Compatibility::BackwardCompatible,
        }
    }

    pub fn push(&mut self, change: C) {
        self.compatibility = self.compatibility.min(change.compat());
        self.changes.push(change);
    }

    pub fn extend<T: IntoIterator<Item = C>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        self.changes.reserve_exact(iter.size_hint().0);
        for change in iter {
            self.push(change);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn compatibility(&self) -> Compatibility {
        self.compatibility
    }
}

impl<C: Change> IntoIterator for ChangeBuf<C> {
    type Item = C;
    type IntoIter = std::vec::IntoIter<C>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.into_iter()
    }
}

impl<'a, C: Change> IntoIterator for &'a ChangeBuf<C> {
    type Item = &'a C;
    type IntoIter = std::slice::Iter<'a, C>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.iter()
    }
}

impl<C: Change> FromIterator<C> for ChangeBuf<C> {
    fn from_iter<T: IntoIterator<Item = C>>(iter: T) -> Self {
        let mut res = Self::new();
        res.extend(iter);
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn compatibility_in_buffer() {
        let mut buf = ChangeBuf::new();
        buf.push(MiniChange(Compatibility::BackwardCompatible));
        buf.push(MiniChange(Compatibility::BackwardCompatible));
        assert_eq!(buf.compatibility(), Compatibility::BackwardCompatible);

        buf.push(MiniChange(Compatibility::Dubious));
        assert_eq!(buf.compatibility(), Compatibility::Dubious);

        buf.push(MiniChange(Compatibility::BackwardCompatible));
        assert_eq!(buf.compatibility(), Compatibility::Dubious);

        buf.push(MiniChange(Compatibility::Breaking));
        assert_eq!(buf.compatibility(), Compatibility::Breaking);

        buf.push(MiniChange(Compatibility::Breaking));
        assert_eq!(buf.compatibility(), Compatibility::Breaking);

        buf.push(MiniChange(Compatibility::BackwardCompatible));
        assert_eq!(buf.compatibility(), Compatibility::Breaking);
    }

    struct MiniChange(Compatibility);
    impl Change for MiniChange {
        fn compat(&self) -> Compatibility {
            self.0
        }
    }
}
