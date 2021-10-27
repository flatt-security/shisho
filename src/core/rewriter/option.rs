use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use crate::core::{
    language::Queryable,
    matcher::{CaptureMap, MatchedItem},
    node::Node,
    node::RootNode,
    ruleset::filter::{PatternWithFilters, RewriteFilter},
};

use super::builder::SnippetBuilder;

#[derive(Debug)]
pub struct RewriteOption<'a, T>
where
    T: Queryable,
{
    root_node: RootNode<'a>,
    filters: &'a Vec<RewriteFilter<T>>,
}

impl<'a, T> RewriteOption<'a, T>
where
    T: Queryable,
{
    pub fn to_string_with<'tree>(
        &'a self,
        captures: &'tree CaptureMap<'tree, Node<'tree>>,
    ) -> Result<String> {
        let segment = SnippetBuilder::<T>::new(&self.root_node, captures)
            .apply_filters(self.filters)?
            .build()?;

        Ok(segment.body)
    }
}

impl<'a, T> From<&'a PatternWithFilters<T>> for RewriteOption<'a, T>
where
    T: Queryable,
{
    fn from(pwf: &'a PatternWithFilters<T>) -> Self {
        let root_node = pwf.pattern.to_root_node();

        Self {
            root_node,
            filters: &pwf.filters,
        }
    }
}

impl<T> PatternWithFilters<T>
where
    T: Queryable,
{
    pub fn as_roption(&'_ self) -> RewriteOption<'_, T> {
        self.into()
    }
}
