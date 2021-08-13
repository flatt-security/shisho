use std::{
    convert::TryFrom,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    constraint::Constraint, language::Queryable, matcher::MatchedItem, query::Query,
    tree::PartialTree,
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleSet {
    pub version: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub language: Language,
    pub message: String,
    pub pattern: String,

    #[serde(default)]
    pub constraints: Vec<RawConstraint>,
}

impl Rule {
    pub fn find<'tree, 'item, T: 'static>(
        &self,
        tree: &'tree PartialTree<'tree, 'tree, T>,
    ) -> Result<Vec<MatchedItem<'item>>>
    where
        T: Queryable,
        'tree: 'item,
    {
        let constraints = self
            .constraints
            .iter()
            .map(|x| Constraint::try_from(x.clone()))
            .collect::<Result<Vec<Constraint<T>>>>()?;

        let query = Query::<T>::try_from(self.pattern.as_str())?;
        let session = tree.matches(&query);

        Ok(session
            .collect()
            .into_iter()
            .filter(|x| x.satisfies_all(&constraints))
            .collect())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RawConstraint {
    pub target: String,
    pub should: RawPredicate,
    pub pattern: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum RawPredicate {
    Match,
    MatchExactly,
    NotMatch,
    NotMatchExactly,

    MatchPartially,
    NotMatchPartially,

    MatchRegex,
    NotMatchRegex,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Hash, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    HCL,
    Go,
}

pub fn from_str(s: &str) -> Result<RuleSet> {
    let rset: RuleSet = serde_yaml::from_str(s)?;
    Ok(rset)
}

pub fn from_reader<P: AsRef<Path>>(ruleset_path: P) -> Result<RuleSet> {
    let f = File::open(ruleset_path)?;
    let rset: RuleSet = serde_yaml::from_reader(f)?;
    Ok(rset)
}

impl<'a> TryFrom<&'a str> for RuleSet {
    type Error = anyhow::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        from_str(value)
    }
}

impl<'a> TryFrom<PathBuf> for RuleSet {
    type Error = anyhow::Error;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        from_reader(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load() {
        let rs = RuleSet::try_from(include_str!("./tests/ruleset/basic.yaml"));
        assert!(rs.is_ok());
    }
}
