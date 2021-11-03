use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use crate::core::{language::Queryable, pattern::Pattern, source::NormalizedSource};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

#[derive(Debug)]
pub struct Constraint<T>
where
    T: Queryable,
{
    pub target: MetavariableId,
    pub predicate: ConstraintPredicate<T>,
}

#[derive(Debug)]
pub enum ConstraintPredicate<T>
where
    T: Queryable,
{
    MatchQuery(Pattern<T>),
    NotMatchQuery(Pattern<T>),
    MatchAnyOfQuery(Vec<Pattern<T>>),
    NotMatchAnyOfQuery(Vec<Pattern<T>>),

    MatchRegex(Regex),
    NotMatchRegex(Regex),
    MatchAnyOfRegex(Vec<Regex>),
    NotMatchAnyOfRegex(Vec<Regex>),

    BeAnyOf(Vec<String>),
    NotBeAnyOf(Vec<String>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RawPatternWithConstraints {
    pub pattern: String,

    #[serde(default)]
    pub constraints: Vec<RawConstraint>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct RawConstraint {
    pub target: String,
    pub should: RawConstraintPredicate,

    pub pattern: Option<String>,
    #[serde(default)]
    pub patterns: Vec<RawPatternWithConstraints>,

    /// deprecated field
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
    pub fn get_patterns(&self) -> Result<Vec<String>> {
        if !self.constraints.is_empty() || self.patterns.iter().any(|p| !p.constraints.is_empty()) {
            Err(anyhow::anyhow!("You can't use `constraints` inside constraints; constraint nesting is removed from v0.5.3."))
        } else {
            match (&self.pattern, &self.patterns) {
                (Some(p), patterns) if patterns.is_empty() => Ok(vec![p.to_string()]),
                (None, patterns) if !patterns.is_empty() => {
                    Ok(patterns.iter().map(|x| x.pattern.clone()).collect())
                }
                (None, patterns) if patterns.is_empty() => Ok(vec![]),
                _ => Err(anyhow::anyhow!(
                    "You can use only one of `pattern` or `patterns`."
                )),
            }
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
pub enum RawConstraintPredicate {
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

impl<T> TryFrom<RawConstraint> for Constraint<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(rc: RawConstraint) -> Result<Self, Self::Error> {
        let predicate = match rc.should {
            RawConstraintPredicate::Match | RawConstraintPredicate::NotMatch => {
                let mut rpwcs = rc.get_patterns()?;
                let mut rrps = rc.get_regex_patterns()?;
                match (rpwcs.len(), rrps.len()) {
                    (1, 0) => {
                        let pc =
                            Pattern::<T>::try_from(NormalizedSource::from(rpwcs.pop().unwrap()))?;
                        if rc.should == RawConstraintPredicate::Match {
                            ConstraintPredicate::MatchQuery(pc)
                        } else if rc.should == RawConstraintPredicate::NotMatch {
                            ConstraintPredicate::NotMatchQuery(pc)
                        } else {
                            unreachable!("invalid state")
                        }
                    }
                    (0, 1) => {
                        let rps = Regex::new(rrps.pop().unwrap().as_str())?;
                        if rc.should == RawConstraintPredicate::Match {
                            ConstraintPredicate::MatchRegex(rps)
                        } else if rc.should == RawConstraintPredicate::NotMatch {
                            ConstraintPredicate::NotMatchRegex(rps)
                        } else {
                            unreachable!("invalid state")
                        }
                    }
                    (0, 0) => {
                        return Err(anyhow::anyhow!(
                            "(not-)match requires either of a pattern or a regex-pattern"
                        ))
                    }
                    (_, _) => {
                        return Err(anyhow::anyhow!(
                            "(not-)match accepts either of a pattern or a regex-pattern"
                        ))
                    }
                }
            }
            RawConstraintPredicate::MatchAnyOf | RawConstraintPredicate::NotMatchAnyOf => {
                let rpwcs = rc.get_patterns()?;
                let rrps = rc.get_regex_patterns()?;
                match (rpwcs.len(), rrps.len()) {
                    (l, 0) if l > 0 => {
                        let pwcs = rpwcs
                            .into_iter()
                            .map(|x| Pattern::<T>::try_from(NormalizedSource::from(x)))
                            .collect::<Result<Vec<Pattern<T>>>>()?;
                        if rc.should == RawConstraintPredicate::MatchAnyOf {
                            ConstraintPredicate::MatchAnyOfQuery(pwcs)
                        } else if rc.should == RawConstraintPredicate::NotMatchAnyOf {
                            ConstraintPredicate::NotMatchAnyOfQuery(pwcs)
                        } else {
                            unreachable!("invalid state")
                        }
                    }
                    (0, r) if r > 0 => {
                        let rps = rrps
                            .into_iter()
                            .map(|x| Ok(Regex::new(x.as_str())?))
                            .collect::<Result<Vec<Regex>>>()?;
                        if rc.should == RawConstraintPredicate::MatchAnyOf {
                            ConstraintPredicate::MatchAnyOfRegex(rps)
                        } else if rc.should == RawConstraintPredicate::NotMatchAnyOf {
                            ConstraintPredicate::NotMatchAnyOfRegex(rps)
                        } else {
                            unreachable!("invalid state")
                        }
                    }
                    (0, 0) => {
                        return Err(anyhow::anyhow!(
                            "(not-)match-any-of requires either of pattern(s) or regex-pattern(s)"
                        ))
                    }
                    (_, _) => {
                        return Err(anyhow::anyhow!(
                            "(not-)match-any-of accepts either of pattern(s) or regex-pattern(s)"
                        ))
                    }
                }
            }
            RawConstraintPredicate::BeAnyOf | RawConstraintPredicate::NotBeAnyOf => {
                if rc.get_patterns().map(|x| x.len()).unwrap_or(0) > 0 {
                    return Err(anyhow::anyhow!("(not-)be-any-of cannot handle pattern(s) and regex-pattern(s). use string(s) instead."));
                }
                if rc.get_regex_patterns().map(|x| x.len()).unwrap_or(0) > 0 {
                    return Err(anyhow::anyhow!("(not-)be-any-of cannot handle pattern(s) and regex-pattern(s). use string(s) instead."));
                }

                let strings = rc.get_strings()?;
                if strings.len() == 0 {
                    return Err(anyhow::anyhow!("(not-)be-any-of requires at least one string specified with `string(s)` attribute"));
                }

                if rc.should == RawConstraintPredicate::BeAnyOf {
                    ConstraintPredicate::BeAnyOf(strings)
                } else if rc.should == RawConstraintPredicate::NotBeAnyOf {
                    ConstraintPredicate::NotBeAnyOf(strings)
                } else {
                    unreachable!("invalid state")
                }
            }

            RawConstraintPredicate::MatchRegex => {
                // TODO (y0n3uchy): deprecate match-regex + patterns
                let patterns = rc.get_patterns()?;
                if patterns.len() == 1 {
                    let r = Regex::new(patterns.get(0).unwrap().as_str())?;
                    ConstraintPredicate::MatchRegex(r)
                } else {
                    return Err(anyhow::anyhow!("match-regex accepts only one pattern once"));
                }
            }
            RawConstraintPredicate::NotMatchRegex => {
                // TODO (y0n3uchy): deprecate match-regex + patterns
                let patterns = rc.get_patterns()?;
                if patterns.len() == 1 {
                    let r = Regex::new(patterns.get(0).unwrap().as_str())?;
                    ConstraintPredicate::NotMatchRegex(r)
                } else {
                    return Err(anyhow::anyhow!(
                        "not-match-regex accepts only one pattern once"
                    ));
                }
            }
        };

        Ok(Constraint {
            target: MetavariableId(rc.target),
            predicate,
        })
    }
}

#[derive(Debug)]
pub struct PatternWithConstraints<T: Queryable> {
    pub pattern: Pattern<T>,
    pub constraints: Vec<Constraint<T>>,
}

impl<T: Queryable> PatternWithConstraints<T> {
    pub fn new(pattern: Pattern<T>, constraints: Vec<Constraint<T>>) -> Self {
        Self {
            pattern,
            constraints,
        }
    }
}

impl<T: Queryable> TryFrom<RawPatternWithConstraints> for PatternWithConstraints<T> {
    type Error = anyhow::Error;

    fn try_from(rpc: RawPatternWithConstraints) -> Result<Self> {
        let pattern = Pattern::<T>::try_from(rpc.pattern.as_str())?;
        let constraints = rpc
            .constraints
            .iter()
            .map(|x| Constraint::try_from(x.clone()))
            .collect::<Result<Vec<Constraint<T>>>>()?;
        Ok(Self {
            pattern,
            constraints,
        })
    }
}
