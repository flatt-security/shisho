use std::{fs::File, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    language::Queryable,
    query::{Query, RawQuery},
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub language: Language,
    pub message: String,
    pub patterns: Vec<RawQueryPattern>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    HCL,
    Go,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawQueryPattern {
    Match(String),
}

pub enum QueryPattern<T>
where
    T: Queryable,
{
    Match(Query<T>),
}

impl RawQueryPattern {
    pub fn to_query<T>(&self) -> Result<QueryPattern<T>>
    where
        T: Queryable,
    {
        match self {
            Self::Match(p) => RawQuery::<T>::new(p)
                .to_query()
                .map(|q| QueryPattern::Match(q)),
        }
    }
}

pub fn from_reader<P: AsRef<Path>>(ruleset_path: P) -> Result<RuleSet> {
    let f = File::open(ruleset_path)?;
    let rset: RuleSet = serde_yaml::from_reader(f)?;
    Ok(rset)
}

pub fn from_str(s: &str) -> Result<RuleSet> {
    let rset: RuleSet = serde_yaml::from_str(s)?;
    Ok(rset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load() {
        let raw = include_str!("./tests/ruleset/basic.yaml");
        let ruleset = from_str(raw);
        println!("{:?}", ruleset);
    }
}
