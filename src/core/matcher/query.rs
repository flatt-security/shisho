use std::convert::TryFrom;

use crate::core::{
    language::Queryable,
    pattern::Pattern,
    ruleset::constraint::{Constraint, PatternWithConstraints},
};

pub struct Query<'a, T: Queryable> {
    pub pattern: Pattern<'a, T>,
    pub constraints: &'a Vec<Constraint<T>>,
}

impl<'a, T> TryFrom<&'a PatternWithConstraints<T>> for Query<'a, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &'a PatternWithConstraints<T>) -> Result<Self, Self::Error> {
        let p = Pattern::<T>::try_from(&value.pattern)?;
        Ok(Self {
            pattern: p,
            constraints: &value.constraints,
        })
    }
}
