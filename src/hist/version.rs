use std::{fmt::Display, str::FromStr};

use itertools::Itertools;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BiVersion {
    pub old: Version,
    pub new: Version,
}

impl BiVersion {
    pub fn from_ref(old: &Version, new: &Version) -> Self {
        Self {
            old: old.clone(),
            new: new.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    pub numbers: Vec<u8>,
}

impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let numbers = s.split('.').map(|s| s.parse::<u8>()).try_collect()?;
        Ok(Self { numbers })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.numbers.iter().join(".");
        f.write_str(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_cmp() {
        let a = Version {
            numbers: vec![3, 1, 0],
        };
        let b = Version {
            numbers: vec![3, 1, 1],
        };
        assert!(a == a);
        assert!(a < b);

        let b = Version {
            numbers: vec![3, 2, 0],
        };
        assert!(a < b);

        let b = Version {
            numbers: vec![4, 0, 0],
        };
        assert!(a < b);
    }

    #[test]
    fn version_str() {
        let a = Version {
            numbers: vec![3, 1, 0],
        };
        assert_eq!(a.to_string(), "3.1.0");
    }
}
