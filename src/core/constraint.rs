use anyhow::Result;
use regex::Regex;
use std::convert::TryFrom;

use crate::core::{
    language::Queryable,
    query::MetavariableId,
    ruleset::{RawConstraint, RawPredicate},
};

use super::ruleset::PatternWithConstraints;

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

    MatchRegex(Regex),
    NotMatchRegex(Regex),
}

impl<T> TryFrom<RawConstraint> for Constraint<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(rc: RawConstraint) -> Result<Self, Self::Error> {
        let mut rpcs = rc.get_patterns()?;
        let predicate = match rc.should {
            RawPredicate::Match => {
                let patterns = rc.get_patterns()?;
                if patterns.len() == 1 {
                    let pc = PatternWithConstraints::<T>::try_from(rpcs.pop().unwrap())?;
                    Predicate::MatchQuery(pc)
                } else {
                    return Err(anyhow::anyhow!("match accepts only one pattern once"));
                }
            }
            RawPredicate::NotMatch => {
                let patterns = rc.get_patterns()?;
                if patterns.len() == 1 {
                    let pc = PatternWithConstraints::<T>::try_from(rpcs.pop().unwrap())?;
                    Predicate::NotMatchQuery(pc)
                } else {
                    return Err(anyhow::anyhow!("not-match accepts only one pattern once"));
                }
            }
            RawPredicate::MatchRegex => {
                let patterns = rc.get_patterns()?;
                if patterns.len() == 1 {
                    let r = Regex::new(patterns.get(0).unwrap().pattern.as_str())?;
                    Predicate::MatchRegex(r)
                } else {
                    return Err(anyhow::anyhow!("match-regex accepts only one pattern once"));
                }
            }
            RawPredicate::NotMatchRegex => {
                let patterns = rc.get_patterns()?;
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
