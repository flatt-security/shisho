mod builder;
mod literal;

use crate::core::{language::Queryable, matcher::MatchedItem, node::RootNode, pattern::Pattern};
use anyhow::Result;

use self::builder::SnippetBuilder;

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
    pub fn into_rewritten_snippet<'tree>(self, item: &'tree MatchedItem) -> Result<String> {
        let builder = SnippetBuilder::new(self, item).build()?;
        Ok(builder.body)
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
