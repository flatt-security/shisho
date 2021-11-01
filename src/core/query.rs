use crate::core::{
    language::Queryable,
    pattern::Pattern,
    ruleset::constraint::{Constraint, PatternWithConstraints},
};

use super::pattern::PatternView;

#[derive(Debug)]
pub struct QueryPattern<'a, T>
where
    T: Queryable,
{
    pub pview: PatternView<'a, T>,
    pattern: &'a Pattern<T>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

impl<'a, T> From<&'a Pattern<T>> for QueryPattern<'a, T>
where
    T: Queryable,
{
    fn from(pattern: &'a Pattern<T>) -> Self {
        let pview = PatternView::from(pattern);
        QueryPattern { pview, pattern }
    }
}

pub struct Query<'a, T: Queryable> {
    pub pattern: QueryPattern<'a, T>,
    pub constraints: &'a Vec<Constraint<T>>,
}

impl<'a, T> From<&'a PatternWithConstraints<T>> for Query<'a, T>
where
    T: Queryable,
{
    fn from(pc: &'a PatternWithConstraints<T>) -> Self {
        Self {
            pattern: (&pc.pattern).into(),
            constraints: &pc.constraints,
        }
    }
}

impl<T> PatternWithConstraints<T>
where
    T: Queryable,
{
    pub fn as_query(&'_ self) -> Query<'_, T> {
        self.into()
    }
}
