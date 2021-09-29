use crate::core::{language::Queryable, node::RootNode, pattern::Pattern};

#[derive(Debug)]
pub struct Query<'a, T>
where
    T: Queryable,
{
    pub root_node: RootNode<'a>,
    pattern: &'a Pattern<T>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

impl<'a, T> From<&'a Pattern<T>> for Query<'a, T>
where
    T: Queryable + 'static,
{
    fn from(pattern: &'a Pattern<T>) -> Self {
        let root_node = pattern.to_root_node();
        Query { root_node, pattern }
    }
}

impl<T> Pattern<T>
where
    T: Queryable + 'static,
{
    pub fn as_query(&'_ self) -> Query<'_, T> {
        self.into()
    }
}
