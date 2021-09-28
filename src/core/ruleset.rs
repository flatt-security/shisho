use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fs::File, path::Path, str::FromStr};
use walkdir::WalkDir;

use crate::core::{
    constraint::Constraint, language::Queryable, matcher::MatchedItem, query::Query, tree::TreeView,
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
        tree: &'tree TreeView<'tree, T>,
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
            let query = Query::<T>::try_from(p.as_str())?;
            matches.extend(
                tree.matches(&query)
                    .filter(|x| x.satisfies_all(&constraints).unwrap_or(false)),
            );
        }
        Ok(matches)
    }

    pub fn get_patterns(&self) -> Result<Vec<String>> {
        match (&self.pattern, &self.patterns) {
            (Some(p), patterns) if patterns.len() == 0 => Ok(vec![p.to_string()]),
            (None, patterns) if patterns.len() > 0 => Ok(patterns.clone()),
            _ => Err(anyhow::anyhow!(
                "You can use only one of `pattern` or `patterns`."
            )),
        }
    }

    pub fn get_rewrite_options(&self) -> Result<Vec<String>> {
        match (&self.rewrite, &self.rewrite_options) {
            (Some(p), patterns) if patterns.len() == 0 => Ok(vec![p.to_string()]),
            (None, patterns) => Ok(patterns.clone()),
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

pub fn from_path<P: AsRef<Path>>(ruleset_path: P) -> Result<Vec<RuleSet>> {
    let ruleset_path: &Path = ruleset_path.as_ref();
    if ruleset_path.is_dir() {
        WalkDir::new(ruleset_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| p.is_file())
            .map(|p| from_filepath(p))
            .collect::<Result<Vec<RuleSet>>>()
    } else {
        Ok(vec![from_filepath(ruleset_path)?])
    }
}

fn from_filepath<P: AsRef<Path>>(p: P) -> Result<RuleSet> {
    let f = File::open(p)?;
    from_reader(f)
}

fn from_reader<R: std::io::Read>(r: R) -> Result<RuleSet> {
    let rset: RuleSet = serde_yaml::from_reader(r)?;
    Ok(rset)
}

impl<'a> TryFrom<&'a str> for RuleSet {
    type Error = anyhow::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        from_str(value)
    }
}
