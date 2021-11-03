#[cfg(test)]
mod test;

pub mod constraint;
pub mod filter;
mod util;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fs::File, path::Path, str::FromStr};
use walkdir::WalkDir;

use crate::core::{
    language::Queryable,
    matcher::MatchedItem,
    node::CSTNode,
    ruleset::{constraint::PatternWithConstraints, filter::RawPatternWithFilters},
    tree::CSTView,
};

use self::{
    constraint::{RawConstraint, RawPatternWithConstraints},
    filter::{RawPatternWithFiltersWrapper, RawRewriteFilter},
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

    pub title: Option<String>,
    pub message: String,

    #[serde(default)]
    pub tags: Vec<Tag>,

    #[serde(default)]
    patterns: Vec<RawPatternWithConstraints>,

    pattern: Option<String>,
    #[serde(default)]
    constraints: Vec<RawConstraint>,

    #[serde(default)]
    rewrite_options: Vec<RawPatternWithFiltersWrapper>,

    rewrite: Option<String>,
    #[serde(default)]
    filters: Vec<RawRewriteFilter>,
}

impl Rule {
    pub fn new(
        id: String,
        language: Language,
        message: String,
        patterns: Vec<RawPatternWithConstraints>,
        rewrite_options: Vec<RawPatternWithFilters>,
        tags: Vec<Tag>,
    ) -> Self {
        Rule {
            id,
            message,
            language,

            title: None,

            patterns,
            rewrite_options: rewrite_options
                .into_iter()
                .map(|x| RawPatternWithFiltersWrapper(x))
                .collect(),

            tags,

            // these params are just for YAMLs
            pattern: None,
            constraints: vec![],

            rewrite: None,
            filters: vec![],
        }
    }

    pub fn find<'tree, T>(
        &self,
        tree: &'tree CSTView<'tree, T>,
    ) -> Result<Vec<MatchedItem<'tree, CSTNode<'tree>>>>
    where
        T: Queryable,
    {
        let patterns = self.get_patterns()?;
        let mut matches = vec![];
        for rpc in patterns {
            let pc = PatternWithConstraints::<T>::try_from(rpc)?;
            let lmatches = tree
                .matches(&pc.as_query())
                .collect::<Result<Vec<MatchedItem<CSTNode>>>>()?;
            matches.extend(lmatches);
        }
        Ok(matches)
    }

    pub fn get_patterns(&self) -> Result<Vec<RawPatternWithConstraints>> {
        match (&self.pattern, &self.patterns) {
            (Some(p), patterns) if patterns.is_empty() => Ok(vec![RawPatternWithConstraints {
                pattern: p.to_string(),
                constraints: self.constraints.clone(),
            }]),
            (None, patterns) if !patterns.is_empty() => Ok(patterns
                .into_iter()
                .map(|p| RawPatternWithConstraints {
                    pattern: p.pattern.to_string(),
                    constraints: [p.constraints.clone(), self.constraints.clone()].concat(),
                })
                .collect()),
            _ => Err(anyhow::anyhow!(
                "You can use only one of `pattern` or `patterns`."
            )),
        }
    }

    pub fn get_rewrite_options(&self) -> Result<Vec<RawPatternWithFilters>> {
        match (&self.rewrite, &self.rewrite_options) {
            (Some(p), patterns) if patterns.is_empty() => Ok(vec![RawPatternWithFilters {
                pattern: p.to_string(),
                filters: self.filters.clone(),
            }]),
            (None, patterns) => Ok(patterns
                .into_iter()
                .map(|p| RawPatternWithFilters {
                    pattern: p.0.pattern.clone(),
                    filters: p.0.filters.clone(),
                })
                .collect()),
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
    pub fn get_severity(&self) -> Severity {
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

impl FromStr for RuleSet {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

pub fn from_path<P: AsRef<Path>>(ruleset_path: P) -> Result<Vec<RuleSet>> {
    let ruleset_path: &Path = ruleset_path.as_ref();
    if ruleset_path.is_dir() {
        WalkDir::new(ruleset_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| {
                p.is_file()
                    && matches!(
                        p.extension().map(|e| e.to_str().unwrap()),
                        Some("yaml") | Some("yml")
                    )
            })
            .map(from_filepath)
            .collect::<Result<Vec<RuleSet>>>()
    } else {
        Ok(vec![from_filepath(ruleset_path)?])
    }
}

pub fn from_filepath<P: AsRef<Path>>(p: P) -> Result<RuleSet> {
    let f = File::open(p)?;
    from_reader(f)
}

pub fn from_reader<R: std::io::Read>(r: R) -> Result<RuleSet> {
    let rset: RuleSet = serde_yaml::from_reader(r)?;
    Ok(rset)
}
