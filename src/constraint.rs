use anyhow::Result;
use regex::Regex;
use std::convert::{TryFrom, TryInto};

use crate::{
    language::Queryable,
    query::{MetavariableId, Query},
    ruleset::{RawConstraint, RawPredicate},
};

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
    MatchExactQuery(Query<T>),
    NotMatchExactQuery(Query<T>),

    MatchPartialQuery(Query<T>),
    NotMatchPartialQuery(Query<T>),

    MatchRegex(Regex),
    NotMatchRegex(Regex),
}

impl<T> Constraint<T> where T: Queryable {}

impl<T> TryFrom<RawConstraint> for Constraint<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(rc: RawConstraint) -> Result<Self, Self::Error> {
        let predicate = match rc.should {
            RawPredicate::Match | RawPredicate::MatchExactly => {
                let p = rc.pattern.as_str().try_into()?;
                Predicate::MatchExactQuery(p)
            }
            RawPredicate::NotMatch | RawPredicate::NotMatchExactly => {
                let p = rc.pattern.as_str().try_into()?;
                Predicate::NotMatchExactQuery(p)
            }
            RawPredicate::MatchPartially => {
                let p = rc.pattern.as_str().try_into()?;
                Predicate::MatchPartialQuery(p)
            }
            RawPredicate::NotMatchPartially => {
                let p = rc.pattern.as_str().try_into()?;
                Predicate::NotMatchPartialQuery(p)
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
