use std::{cell::RefCell, rc::Rc};

use crate::core::{
    language::Queryable,
    matcher::MatchedItem,
    node::Node,
    node::RootNode,
    pattern::{Pattern, PatternWithConstraints},
    query::MetavariableId,
};

use super::{builder::SnippetBuilder, node::RewritableNode};

#[derive(Debug)]
pub struct RewriteOption<'a, T>
where
    T: Queryable,
{
    root_node: RootNode<'a>,

    pub(crate) pattern: &'a Pattern<T>,
    pub filters: Vec<RewriteFilter<T>>,
}

#[derive(Debug)]
pub struct RewriteFilter<T>
where
    T: Queryable,
{
    pub target: MetavariableId,
    pub predicate: RewriteFilterPredicate<T>,
}

#[derive(Debug)]
pub struct PatternWithFilters<T: Queryable> {
    pub pattern: Pattern<T>,
    pub filters: Vec<RewriteFilter<T>>,
}

#[derive(Debug)]
pub enum RewriteFilterPredicate<T>
where
    T: Queryable,
{
    ReplaceWithQuery((PatternWithConstraints<T>, PatternWithFilters<T>)),
}

impl<'a, T> RewriteOption<'a, T>
where
    T: Queryable,
{
    pub fn to_builder<'tree>(
        &'a self,
        item: &'tree MatchedItem<'tree, Node<'tree>>,
    ) -> SnippetBuilder<'tree, T> {
        let source = self.pattern.source.clone();
        let source = Rc::new(RefCell::new(source));

        let rnode: &Node = (&self.root_node).into();
        let rnode = RewritableNode::from_node(rnode, source.clone());

        SnippetBuilder::<T>::new(rnode, source, self.pattern.with_extra_newline, item)
    }
}

impl<'a, T> From<&'a Pattern<T>> for RewriteOption<'a, T>
where
    T: Queryable,
{
    fn from(pattern: &'a Pattern<T>) -> Self {
        let root_node = pattern.to_root_node();

        todo!("set filters");
        Self {
            pattern,
            root_node,
            filters: vec![],
        }
    }
}

impl<T> Pattern<T>
where
    T: Queryable,
{
    pub fn as_roption(&'_ self) -> RewriteOption<'_, T> {
        self.into()
    }
}
