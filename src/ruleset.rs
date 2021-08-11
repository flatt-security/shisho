use std::{
    convert::TryFrom,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

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
    pub constraints: Vec<RawConstraint>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RawConstraint {
    pub target: String,
    pub predicate: RawPredicate,
    pub pattern: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RawPredicate {
    Match,
    NotMatch,

    MatchRegex,
    NotMatchRegex,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
        let raw = include_str!("./tests/ruleset/basic.yaml");
        let ruleset = from_str(raw);

        assert!(ruleset.is_ok());
    }
}
