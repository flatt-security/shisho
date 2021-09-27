use anyhow::Result;
use regex::Regex;
use std::convert::{TryFrom, TryInto};

use crate::core::{
    language::Queryable,
    query::{MetavariableId, Query},
    ruleset::{RawConstraint, RawPredicate},
};

#[derive(Debug)]
pub struct Constraint<'a, T>
where
    T: Queryable,
{
    pub target: MetavariableId,
    pub predicate: Predicate<'a, T>,
}

#[derive(Debug)]
pub enum Predicate<'a, T>
where
    T: Queryable,
{
    MatchQuery(Query<'a, T>),
    NotMatchQuery(Query<'a, T>),

    MatchRegex(Regex),
    NotMatchRegex(Regex),
}

impl<'a, T> Constraint<'a, T> where T: Queryable {}

impl<'a, T> TryFrom<RawConstraint> for Constraint<'a, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(rc: RawConstraint) -> Result<Self, Self::Error> {
        let predicate = match rc.should {
            RawPredicate::Match => {
                let p = rc.pattern.try_into()?;
                Predicate::MatchQuery(p)
            }
            RawPredicate::NotMatch => {
                let p = rc.pattern.try_into()?;
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
