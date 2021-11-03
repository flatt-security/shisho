use crate::core::{
    language::Queryable,
    pattern::{Pattern, PatternView},
    ruleset::constraint::{Constraint, PatternWithConstraints},
};

pub struct Query<'a, T: Queryable> {
    pub pattern: PatternView<'a, T>,
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
    pub fn without_constraints(pattern: Pattern<T>) -> Self {
        Self {
            pattern,
            constraints: vec![],
        }
    }

    pub fn as_query(&'_ self) -> Query<'_, T> {
        self.into()
    }
}
