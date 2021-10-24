#[cfg(test)]
mod test;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fs::File, path::Path, str::FromStr};
use walkdir::WalkDir;

use crate::core::{
    language::Queryable, matcher::MatchedItem, node::Node, pattern::PatternWithConstraints,
    tree::RefTreeView,
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
    rewrite_options: Vec<String>,
    rewrite: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RawPatternWithConstraints {
    pub pattern: String,

    #[serde(default)]
    pub constraints: Vec<RawConstraint>,
}

impl Rule {
    pub fn new(
        id: String,
        language: Language,
        message: String,
        patterns: Vec<RawPatternWithConstraints>,
        rewrite_options: Vec<String>,
        tags: Vec<Tag>,
    ) -> Self {
        Rule {
            id,
            message,
            language,

            title: None,

            patterns,
            constraints: vec![],
            rewrite_options,
            tags,

            // these params are just for YAMLs
            pattern: None,
            rewrite: None,
        }
    }

    pub fn find<'tree, 'item, T>(
        &self,
        tree: &'tree RefTreeView<'tree, T, Node<'tree>>,
    ) -> Result<Vec<MatchedItem<'item, Node<'tree>>>>
    where
        T: Queryable,
        'tree: 'item,
    {
        let patterns = self.get_patterns()?;
        let mut matches = vec![];
        for rpc in patterns {
            let pc = PatternWithConstraints::<T>::try_from(rpc)?;
            let lmatches = tree
                .matches(&pc.as_query())
                .collect::<Result<Vec<MatchedItem<Node>>>>()?;
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

    pub fn get_rewrite_options(&self) -> Result<Vec<String>> {
        match (&self.rewrite, &self.rewrite_options) {
            (Some(p), patterns) if patterns.is_empty() => Ok(vec![p.to_string()]),
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

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct RawConstraint {
    pub target: String,
    pub should: RawPredicate,

    pub pattern: Option<String>,
    #[serde(default)]
    pub patterns: Vec<RawPatternWithConstraints>,

    #[serde(default)]
    pub constraints: Vec<RawConstraint>,

    pub string: Option<String>,
    #[serde(default)]
    pub strings: Vec<String>,

    pub regex_pattern: Option<String>,
    #[serde(default)]
    pub regex_patterns: Vec<String>,
}

impl RawConstraint {
    pub fn get_pattern_with_constraints(&self) -> Result<Vec<RawPatternWithConstraints>> {
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
            (None, patterns) if patterns.is_empty() => Ok(vec![]),
            _ => Err(anyhow::anyhow!(
                "You can use only one of `pattern` or `patterns`."
            )),
        }
    }

    pub fn get_strings(&self) -> Result<Vec<String>> {
        match (&self.string, &self.strings) {
            (Some(p), patterns) if patterns.is_empty() => Ok(vec![p.to_string()]),
            (None, patterns) if !patterns.is_empty() => Ok(patterns.clone()),
            (None, patterns) if patterns.is_empty() => Ok(vec![]),
            _ => Err(anyhow::anyhow!(
                "You can use only one of `string` or `strings`."
            )),
        }
    }

    pub fn get_regex_patterns(&self) -> Result<Vec<String>> {
        match (&self.regex_pattern, &self.regex_patterns) {
            (Some(p), patterns) if patterns.is_empty() => Ok(vec![p.to_string()]),
            (None, patterns) if !patterns.is_empty() => Ok(patterns.clone()),
            (None, patterns) if patterns.is_empty() => Ok(vec![]),
            _ => Err(anyhow::anyhow!(
                "You can use only one of `regex-pattern` or `regex-patterns`."
            )),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum RawPredicate {
    // takes either of a regex or a pattern
    Match,
    NotMatch,

    // takes either of regex-patterns or patterns
    MatchAnyOf,
    NotMatchAnyOf,

    // TODO: mark this as deprecated
    // takes only a regex
    MatchRegex,
    NotMatchRegex,

    // takes only string
    BeAnyOf,
    NotBeAnyOf,
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
