use std::{
    convert::TryFrom,
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::core::{
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

    pub rewrite: Option<String>,
}

impl Rule {
    pub fn find<'tree, 'item, T: 'static>(
        &self,
        tree: &'tree PartialTree<'tree, T>,
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
            .filter(|x| x.satisfies_all(&constraints).unwrap_or(false))
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
    NotMatch,

    MatchRegex,
    NotMatchRegex,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Hash, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    HCL,
    Dockerfile,
    Go,
}

impl FromStr for Language {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

impl Language {
    pub fn from_str(s: &str) -> Result<Self> {
        let rset: Self = serde_yaml::from_str(s)?;
        Ok(rset)
    }
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
