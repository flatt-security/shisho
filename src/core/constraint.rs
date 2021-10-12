use anyhow::Result;
use regex::Regex;
use std::convert::TryFrom;

use crate::core::{
    language::Queryable,
    query::MetavariableId,
    ruleset::{RawConstraint, RawPredicate},
};

use super::pattern::PatternWithConstraints;

#[derive(Debug)]
pub struct Constraint<T>
where
    T: Queryable,
{
    pub target: MetavariableId,
    pub predicate: Predicate<T>,
}

#[derive(Debug)]
pub enum Predicate<T>
where
    T: Queryable,
{
    MatchQuery(PatternWithConstraints<T>),
    NotMatchQuery(PatternWithConstraints<T>),
    MatchAnyOfQuery(Vec<PatternWithConstraints<T>>),
    NotMatchAnyOfQuery(Vec<PatternWithConstraints<T>>),

    MatchRegex(Regex),
    NotMatchRegex(Regex),
    MatchAnyOfRegex(Vec<Regex>),
    NotMatchAnyOfRegex(Vec<Regex>),

    BeAnyOf(Vec<String>),
    NotBeAnyOf(Vec<String>),
}

impl<T> TryFrom<RawConstraint> for Constraint<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(rc: RawConstraint) -> Result<Self, Self::Error> {
        let predicate = match rc.should {
            RawPredicate::Match | RawPredicate::NotMatch => {
                let mut rpwcs = rc.get_pattern_with_constraints()?;
                let mut rrps = rc.get_regex_patterns()?;
                match (rpwcs.len(), rrps.len()) {
                    (1, 0) => {
                        let pc = PatternWithConstraints::<T>::try_from(rpwcs.pop().unwrap())?;
                        if rc.should == RawPredicate::Match {
                            Predicate::MatchQuery(pc)
                        } else if rc.should == RawPredicate::NotMatch {
                            Predicate::NotMatchQuery(pc)
                        } else {
                            unreachable!("invalid state")
                        }
                    }
                    (0, 1) => {
                        let rps = Regex::new(rrps.pop().unwrap().as_str())?;
                        if rc.should == RawPredicate::Match {
                            Predicate::MatchRegex(rps)
                        } else if rc.should == RawPredicate::NotMatch {
                            Predicate::NotMatchRegex(rps)
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
            RawPredicate::MatchAnyOf | RawPredicate::NotMatchAnyOf => {
                let rpwcs = rc.get_pattern_with_constraints()?;
                let rrps = rc.get_regex_patterns()?;
                match (rpwcs.len(), rrps.len()) {
                    (l, 0) if l > 0 => {
                        let pwcs = rpwcs
                            .into_iter()
                            .map(|x| PatternWithConstraints::<T>::try_from(x))
                            .collect::<Result<Vec<PatternWithConstraints<T>>>>()?;
                        if rc.should == RawPredicate::MatchAnyOf {
                            Predicate::MatchAnyOfQuery(pwcs)
                        } else if rc.should == RawPredicate::NotMatchAnyOf {
                            Predicate::NotMatchAnyOfQuery(pwcs)
                        } else {
                            unreachable!("invalid state")
                        }
                    }
                    (0, r) if r > 0 => {
                        let rps = rrps
                            .into_iter()
                            .map(|x| Ok(Regex::new(x.as_str())?))
                            .collect::<Result<Vec<Regex>>>()?;
                        if rc.should == RawPredicate::MatchAnyOf {
                            Predicate::MatchAnyOfRegex(rps)
                        } else if rc.should == RawPredicate::NotMatchAnyOf {
                            Predicate::NotMatchAnyOfRegex(rps)
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
            RawPredicate::BeAnyOf | RawPredicate::NotBeAnyOf => {
                if rc
                    .get_pattern_with_constraints()
                    .map(|x| x.len())
                    .unwrap_or(0)
                    > 0
                {
                    return Err(anyhow::anyhow!("(not-)be-any-of cannot handle pattern(s) and regex-pattern(s). use string(s) instead."));
                }
                if rc.get_regex_patterns().map(|x| x.len()).unwrap_or(0) > 0 {
                    return Err(anyhow::anyhow!("(not-)be-any-of cannot handle pattern(s) and regex-pattern(s). use string(s) instead."));
                }

                if rc.should == RawPredicate::BeAnyOf {
                    Predicate::BeAnyOf(rc.get_strings()?)
                } else if rc.should == RawPredicate::NotBeAnyOf {
                    Predicate::NotBeAnyOf(rc.get_strings()?)
                } else {
                    unreachable!("invalid state")
                }
            }

            RawPredicate::MatchRegex => {
                // TODO (y0n3uchy): deprecate match-regex + patterns
                let patterns = rc.get_pattern_with_constraints()?;
                if patterns.len() == 1 {
                    let r = Regex::new(patterns.get(0).unwrap().pattern.as_str())?;
                    Predicate::MatchRegex(r)
                } else {
                    return Err(anyhow::anyhow!("match-regex accepts only one pattern once"));
                }
            }
            RawPredicate::NotMatchRegex => {
                // TODO (y0n3uchy): deprecate match-regex + patterns
                let patterns = rc.get_pattern_with_constraints()?;
                if patterns.len() == 1 {
                    let r = Regex::new(patterns.get(0).unwrap().pattern.as_str())?;
                    Predicate::NotMatchRegex(r)
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
