use anyhow::Result;
use regex::Regex;
use std::convert::TryFrom;

use crate::core::{
    language::Queryable,
    query::MetavariableId,
    ruleset::{RawConstraint, RawPredicate},
};

use super::pattern::Pattern;

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
    MatchQuery(Pattern<T>),
    NotMatchQuery(Pattern<T>),

    MatchRegex(Regex),
    NotMatchRegex(Regex),
}

impl<T> TryFrom<RawConstraint> for Constraint<T>
where
    T: Queryable + 'static,
{
    type Error = anyhow::Error;

    fn try_from(rc: RawConstraint) -> Result<Self, Self::Error> {
        let predicate = match rc.should {
            RawPredicate::Match => {
                let p = Pattern::<T>::try_from(rc.pattern.as_str())?;
                Predicate::MatchQuery(p)
            }
            RawPredicate::NotMatch => {
                let p = Pattern::<T>::try_from(rc.pattern.as_str())?;
                Predicate::NotMatchQuery(p)
            }
            RawPredicate::MatchRegex => {
                let r = Regex::new(rc.pattern.as_str())?;
                Predicate::MatchRegex(r)
            }
            RawPredicate::NotMatchRegex => {
                let r = Regex::new(rc.pattern.as_str())?;
                Predicate::NotMatchRegex(r)
            }
        };

        Ok(Constraint {
            target: MetavariableId(rc.target),
            predicate,
        })
    }
}
