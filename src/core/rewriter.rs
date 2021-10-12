mod snippet;

use crate::core::{language::Queryable, matcher::MatchedItem, node::RootNode, pattern::Pattern};
use anyhow::Result;

use self::snippet::SnippetBuilder;

pub struct RewriteOption<'a, T>
where
    T: Queryable,
{
    pub root_node: RootNode<'a>,
    pattern: &'a Pattern<T>,
}

impl<'a, T> RewriteOption<'a, T>
where
    T: Queryable,
{
    pub fn to_rewritten_snippet<'tree>(&self, item: &'tree MatchedItem) -> Result<String> {
        Ok(SnippetBuilder::new(self, item)
            .from_root(&self.root_node)?
            .body)
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
