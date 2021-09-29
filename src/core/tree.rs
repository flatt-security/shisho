use crate::core::{
    language::Queryable,
    matcher::{MatchedItem, TreeMatcher},
    query::Query,
};
use anyhow::{anyhow, Result};
use std::{
    collections::VecDeque,
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

use super::{
    node::{Node, NodeType, RootNode},
    source::NormalizedSource,
};

pub struct Tree<'tree, T> {
    pub source: Vec<u8>,
    with_extra_newline: bool,

    tstree: tree_sitter::Tree,
    _marker: PhantomData<&'tree T>,
}

impl<'tree, T> Tree<'tree, T>
where
    T: Queryable,
{
    pub fn to_root_node(&'_ self) -> RootNode<'_> {
        RootNode::from_tstree(&self.tstree, &self.source, self.with_extra_newline)
    }
}

impl<'tree, T> TryFrom<NormalizedSource> for Tree<'tree, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(nsource: NormalizedSource) -> Result<Self, anyhow::Error> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(T::target_language())
            .expect("Error loading hcl grammar");

        let tstree = parser
            .parse(nsource.as_ref(), None)
            .ok_or(anyhow!("failed to load the code"))?;

        let with_extra_newline = nsource.with_extra_newline();
        Ok(Tree {
            source: nsource.into(),
            with_extra_newline,

            tstree,
            _marker: PhantomData,
        })
    }
}

impl<'tree, T> TryFrom<&str> for Tree<'tree, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(nsource: &str) -> Result<Self, anyhow::Error> {
        let nsource = NormalizedSource::from(nsource);
        nsource.try_into()
    }
}

pub struct TreeView<'tree, T> {
    pub view_root: Node<'tree>,
    pub source: &'tree [u8],
    _marker: PhantomData<T>,
}

impl<'tree, T> TreeView<'tree, T>
where
    T: Queryable,
{
    pub fn new(view_root: Node<'tree>, source: &'tree [u8]) -> TreeView<'tree, T> {
        TreeView {
            view_root,
            source,
            _marker: PhantomData,
        }
    }

    pub fn matches<'query>(
        &'tree self,
        query: &'query Query<'query, T>,
    ) -> impl Iterator<Item = MatchedItem<'tree>> + 'query
    where
        'tree: 'query,
    {
        TreeMatcher::new(self, query)
    }

    pub fn traverse(&'tree self) -> TreeTreverser<'tree> {
        TreeTreverser::new(&self.view_root)
    }
}

impl<'tree, T> From<&'tree Tree<'tree, T>> for TreeView<'tree, T>
where
    T: Queryable,
{
    fn from(t: &'tree Tree<'tree, T>) -> Self {
        TreeView {
            view_root: t.to_root_node().into(),
            source: &t.source,
            _marker: PhantomData,
        }
    }
}

impl<'tree, T> From<Node<'tree>> for TreeView<'tree, T>
where
    T: Queryable,
{
    fn from(t: Node<'tree>) -> Self {
        let source = t.source;
        TreeView {
            view_root: t,
            source,
            _marker: PhantomData,
        }
    }
}

#[allow(unused)]
pub trait TreeVisitor<'tree, T>
where
    T: Queryable,
{
    type Output;

    fn walk_leaf_named_node(&self, node: &Node) -> Result<Self::Output, anyhow::Error>;

    fn walk_leaf_unnamed_node(&self, node: &Node) -> Result<Self::Output, anyhow::Error>;

    fn walk_intermediate_node(&self, node: &Node) -> Result<Self::Output, anyhow::Error> {
        let children = node
            .children
            .iter()
            .map(|child| self.handle_node(child))
            .collect::<Result<Vec<Self::Output>, anyhow::Error>>()?;

        self.flatten_intermediate_node(node, children)
    }

    fn flatten_intermediate_node(
        &self,
        node: &Node,
        children: Vec<Self::Output>,
    ) -> Result<Self::Output, anyhow::Error>;

    fn walk_ellipsis(&self, node: &Node) -> Result<Self::Output, anyhow::Error>;

    fn walk_ellipsis_metavariable(
        &self,
        node: &Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error>;

    fn walk_metavariable(
        &self,
        node: &Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error>;

    fn handle_node(&self, node: &Node) -> Result<Self::Output, anyhow::Error> {
        match node.kind() {
            NodeType::Ellipsis => self.walk_ellipsis(node),
            NodeType::EllipsisMetavariable(mid) => self.walk_ellipsis_metavariable(node, &mid.0),
            NodeType::Metavariable(mid) => self.walk_metavariable(node, &mid.0),
            _ if (self.children_of(node) == 0 || T::is_leaf_like(node)) => {
                if node.is_named() {
                    self.walk_leaf_named_node(node)
                } else {
                    self.walk_leaf_unnamed_node(node)
                }
            }
            _ => self.walk_intermediate_node(node),
        }
    }

    fn children_of(&self, node: &Node) -> usize {
        node.children.len()
    }
}

pub struct TreeTreverser<'a> {
    queue: VecDeque<(usize, &'a Node<'a>)>,
}

impl<'a> TreeTreverser<'a> {
    #[inline]
    pub fn new(root: &'a Node<'a>) -> Self {
        Self {
            queue: VecDeque::from(vec![(0, root)]),
        }
    }
}

impl<'a> Iterator for TreeTreverser<'a> {
    type Item = (usize, &'a Node<'a>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((depth, node)) = self.queue.pop_front() {
            let children = node.children.iter();
            self.queue.extend(children.map(|child| (depth + 1, child)));

            Some((depth, node))
        } else {
            None
        }
    }
}
