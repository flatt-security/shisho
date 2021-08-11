use std::convert::{TryFrom, TryInto};

use anyhow::Result;

use crate::{
    language::Queryable,
    query::{MetavariableId, Query},
    ruleset::{RawConstraint, RawPredicate},
};

#[derive(Debug, PartialEq)]
pub struct Constraint<T>
where
    T: Queryable,
{
    target: MetavariableId,
    predicate: Predicate<T>,
}

#[derive(Debug, PartialEq)]
pub enum Predicate<T>
where
    T: Queryable,
{
    MatchQuery(Query<T>),
    NotMatchQuery(Query<T>),

    MatchRegex(String),
    NotMatchRegex(String),
}

impl<T> Constraint<T> where T: Queryable {}

impl<T> TryFrom<RawConstraint> for Constraint<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(rc: RawConstraint) -> Result<Self, Self::Error> {
        let predicate = match rc.should {
            RawPredicate::Match => {
                let p = rc.pattern.as_str().try_into()?;
                Predicate::MatchQuery(p)
            }
            RawPredicate::NotMatch => {
                let p = rc.pattern.as_str().try_into()?;
                Predicate::NotMatchQuery(p)
            }
            RawPredicate::MatchRegex => Predicate::MatchRegex(rc.pattern),
            RawPredicate::NotMatchRegex => Predicate::NotMatchRegex(rc.pattern),
        };

        Ok(Constraint {
            target: MetavariableId(rc.target),
            predicate,
        })
    }
}
