use std::{fs::File, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub language: Language,
    pub message: String,
    pub pattern: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    HCL,
    Go,
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
