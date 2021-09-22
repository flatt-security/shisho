use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

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

    #[serde(default)]
    patterns: Vec<String>,
    pattern: Option<String>,

    #[serde(default)]
    pub tags: Vec<Tag>,

    #[serde(default)]
    pub constraints: Vec<RawConstraint>,

    #[serde(default)]
    rewrite_options: Vec<String>,
    rewrite: Option<String>,
}

impl Rule {
    pub fn new(
        id: String,
        language: Language,
        message: String,
        patterns: Vec<String>,
        constraints: Vec<RawConstraint>,
        rewrite_options: Vec<String>,
        tags: Vec<Tag>,
    ) -> Self {
        Rule {
            id,
            message,
            language,
            constraints,

            patterns,
            pattern: None,

            rewrite_options,
            rewrite: None,

            tags,
        }
    }

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

        let patterns = self.get_patterns()?;
        let mut matches = vec![];
        for p in patterns {
            let query = Query::<T>::try_from(p)?;
            matches.extend(
                tree.matches(&query)
                    .filter(|x| x.satisfies_all(&constraints).unwrap_or(false)),
            );
        }
        Ok(matches)
    }

    pub fn get_patterns(&self) -> Result<Vec<&str>> {
        match (&self.pattern, &self.patterns) {
            (Some(p), patterns) if patterns.len() == 0 => Ok(vec![&p]),
            (None, patterns) if patterns.len() > 0 => {
                Ok(patterns.iter().map(|x| x.as_str()).collect())
            }
            _ => Err(anyhow::anyhow!(
                "You can use only one of `pattern` or `patterns`."
            )),
        }
    }

    pub fn get_rewrite_options(&self) -> Result<Vec<&str>> {
        match (&self.rewrite, &self.rewrite_options) {
            (Some(p), patterns) if patterns.len() == 0 => Ok(vec![&p]),
            (None, patterns) => Ok(patterns.iter().map(|x| x.as_str()).collect()),
            _ => Err(anyhow::anyhow!(
                "You can use only one of `rewrite` or `rewrite_options`."
            )),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Tag(String);

impl Tag {
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
    Unknown,
}

impl Rule {
    pub fn get_level(&self) -> Severity {
        self.tags
            .iter()
            .find_map(|t| match t.clone().into_inner().to_lowercase().as_str() {
                "low" => Some(Severity::Low),
                "medium" => Some(Severity::Medium),
                "high" => Some(Severity::High),
                "critical" => Some(Severity::Critical),
                _ => None,
            })
            .unwrap_or(Severity::Unknown)
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
