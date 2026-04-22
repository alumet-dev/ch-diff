//! Filtering to discard what we're not interested in.

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::Context;
use rustc_hash::FxHashSet;

pub struct DiffFilter {
    config: FilterConfig,
}

enum FilterConfig {
    Allow,
    Whitelist(FxHashSet<String>),
}

impl DiffFilter {
    pub fn allow_everything() -> Self {
        Self {
            config: FilterConfig::Allow,
        }
    }

    pub fn from_whitelist(whitelist: impl Into<FxHashSet<String>>) -> Self {
        Self {
            config: FilterConfig::Whitelist(whitelist.into()),
        }
    }

    pub fn parse_whitelist_file(file: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut set = FxHashSet::default();
        let path = file.as_ref();
        let file = File::open(path).with_context(|| format!("failed to open file {path:?}"))?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let line = line.trim_ascii();
            if !line.is_empty() && !line.starts_with("#") {
                set.insert(line.to_owned());
            }
        }
        Ok(Self::from_whitelist(set))
    }

    pub fn accepts(&self, name: &str) -> bool {
        match &self.config {
            FilterConfig::Allow => true,
            FilterConfig::Whitelist(set) => set.contains(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn whitelist_from_file() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let whitelist_path = dir.path().join("whitelist.txt");
        std::fs::write(
            &whitelist_path,
            indoc! { "
            z
            item_2
            super_duper_function
            "
            },
        )?;
        let filter = DiffFilter::parse_whitelist_file(&whitelist_path)?;
        assert!(filter.accepts("item_2"));
        assert!(filter.accepts("z"));
        assert!(filter.accepts("super_duper_function"));
        assert!(!filter.accepts("super_duper_function_"));
        assert!(!filter.accepts("Z"));
        assert!(!filter.accepts("not me"));
        Ok(())
    }
}
