use crate::core::{language::Queryable, node::RootNode, pattern::Pattern};

use super::{constraint::Constraint, pattern::PatternWithConstraints};

#[derive(Debug)]
pub struct QueryPattern<'a, T>
where
    T: Queryable,
{
    pub root_node: RootNode<'a>,
    pattern: &'a Pattern<T>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

impl<'a, T> From<&'a Pattern<T>> for QueryPattern<'a, T>
where
    T: Queryable,
{
    fn from(pattern: &'a Pattern<T>) -> Self {
        let root_node = pattern.to_root_node();
        QueryPattern { root_node, pattern }
    }
}

pub struct Query<'a, T: Queryable> {
    pub query: QueryPattern<'a, T>,
    pub constraints: &'a Vec<Constraint<T>>,
}

impl<'a, T> From<&'a PatternWithConstraints<T>> for Query<'a, T>
where
    T: Queryable,
{
    fn from(pc: &'a PatternWithConstraints<T>) -> Self {
        Self {
            query: (&pc.pattern).into(),
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
