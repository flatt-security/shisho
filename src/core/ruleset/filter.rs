use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use crate::core::{language::Queryable, pattern::Pattern, query::MetavariableId};

use super::constraint::{PatternWithConstraints, RawConstraint, RawPatternWithConstraints};

#[derive(Debug)]
pub struct RewriteFilter<T>
where
    T: Queryable,
{
    pub target: MetavariableId,
    pub predicate: RewriteFilterPredicate<T>,
}

#[derive(Debug)]
pub enum RewriteFilterPredicate<T>
where
    T: Queryable,
{
    ReplaceWithQuery((Vec<PatternWithConstraints<T>>, Pattern<T>)),
}

#[derive(Debug)]
pub struct PatternWithFilters<T: Queryable> {
    pub pattern: Pattern<T>,
    pub filters: Vec<RewriteFilter<T>>,
}

impl<T: Queryable> PatternWithFilters<T> {
    pub fn new(pattern: Pattern<T>, filters: Vec<RewriteFilter<T>>) -> Self {
        Self { pattern, filters }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RawPatternWithFilters {
    pub pattern: String,

    #[serde(default)]
    pub filters: Vec<RawRewriteFilter>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum RawRewriteFilter {
    Replace(RawReplaceFilter),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub struct RawReplaceFilter {
    pub target: String,

    pub pattern: Option<String>,
    #[serde(default)]
    pub constraints: Vec<RawConstraint>,

    #[serde(default)]
    pub patterns: Vec<RawPatternWithConstraints>,

    pub to: String,
}

impl RawReplaceFilter {
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
}

impl<T: Queryable> TryFrom<RawPatternWithFilters> for PatternWithFilters<T> {
    type Error = anyhow::Error;

    fn try_from(rpc: RawPatternWithFilters) -> Result<Self> {
        let pattern = Pattern::<T>::try_from(rpc.pattern.as_str())?;
        let filters = rpc
            .filters
            .iter()
            .map(|x| RewriteFilter::try_from(x.clone()))
            .collect::<Result<Vec<RewriteFilter<T>>>>()?;
        Ok(Self { pattern, filters })
    }
}

impl<T> TryFrom<RawRewriteFilter> for RewriteFilter<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(rc: RawRewriteFilter) -> Result<Self, Self::Error> {
        match rc {
            RawRewriteFilter::Replace(r) => {
                let from_patterns = r
                    .get_pattern_with_constraints()?
                    .into_iter()
                    .map(|x| PatternWithConstraints::<T>::try_from(x))
                    .collect::<Result<Vec<PatternWithConstraints<T>>>>()?;

                let to_pattern = Pattern::<T>::try_from(r.to.as_str())?;
                Ok(RewriteFilter {
                    target: MetavariableId(r.target),
                    predicate: RewriteFilterPredicate::ReplaceWithQuery((
                        from_patterns,
                        to_pattern,
                    )),
                })
            }
        }
    }
}
