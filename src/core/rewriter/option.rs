use crate::core::{
    language::Queryable, matcher::MatchedItem, node::Node, node::RootNode, pattern::Pattern,
};

use super::builder::SnippetBuilder;

pub struct RewriteOption<'a, T>
where
    T: Queryable,
{
    pub root_node: RootNode<'a>,
    pub(crate) pattern: &'a Pattern<T>,
}

impl<'a, T> RewriteOption<'a, T>
where
    T: Queryable,
{
    pub fn into_builder<'tree>(
        self,
        item: &'tree MatchedItem<'tree, Node<'tree>>,
    ) -> SnippetBuilder<T> {
        SnippetBuilder::<T>::new(self, item)
    }
}

impl<'a, T> From<&'a Pattern<T>> for RewriteOption<'a, T>
where
    T: Queryable,
{
    fn from(pattern: &'a Pattern<T>) -> Self {
        let root_node = pattern.to_root_node();
        Self { pattern, root_node }
    }
}

impl<T> Pattern<T>
where
    T: Queryable,
{
    pub fn as_rewrite_option(&'_ self) -> RewriteOption<'_, T> {
        self.into()
    }
}
