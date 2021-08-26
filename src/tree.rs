use crate::{
    language::Queryable,
    matcher::{MatchedItem, QueryMatcher},
    query::{
        Query, SHISHO_NODE_ELLIPSIS, SHISHO_NODE_ELLIPSIS_METAVARIABLE, SHISHO_NODE_METAVARIABLE,
        SHISHO_NODE_METAVARIABLE_NAME,
    },
};
use anyhow::{anyhow, Result};
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

pub struct Tree<T> {
    pub(crate) tstree: tree_sitter::Tree,

    raw: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T> Tree<T>
where
    T: Queryable,
{
    pub fn new(tree: tree_sitter::Tree, raw: Vec<u8>) -> Tree<T> {
        Tree {
            tstree: tree,
            raw,
            _marker: PhantomData,
        }
    }

    pub fn to_partial<'tree>(&'tree self) -> PartialTree<'tree, T> {
        PartialTree::new(self.tstree.root_node(), self.raw.as_slice())
    }
}

pub struct PartialTree<'tree, T> {
    pub(crate) root: tree_sitter::Node<'tree>,

    raw: &'tree [u8],
    _marker: PhantomData<T>,
}

impl<'tree, T> PartialTree<'tree, T>
where
    T: Queryable,
{
    pub fn new(top: tree_sitter::Node<'tree>, raw: &'tree [u8]) -> PartialTree<'tree, T> {
        PartialTree {
            root: top,
            raw,
            _marker: PhantomData,
        }
    }

    pub fn matches<'query>(
        &'tree self,
        query: &'query Query<T>,
    ) -> impl Iterator<Item = MatchedItem<'tree>> + 'query
    where
        'tree: 'query,
    {
        QueryMatcher::new(self, query).into_iter()
    }

    pub fn value_of(&self, node: &tree_sitter::Node<'tree>) -> &str {
        node.utf8_text(self.raw).unwrap()
    }
}

impl<'tree, T> AsRef<[u8]> for PartialTree<'tree, T>
where
    T: Queryable,
{
    fn as_ref(&self) -> &[u8] {
        self.raw
    }
}

#[derive(Debug, PartialEq)]
pub struct RawTree<T>
where
    T: Queryable,
{
    raw_bytes: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<'a, T> From<&'a str> for RawTree<T>
where
    T: Queryable,
{
    fn from(value: &'a str) -> Self {
        let value = value.to_string();
        RawTree {
            raw_bytes: if value.as_bytes().len() != 0
                && value.as_bytes()[value.as_bytes().len() - 1] != b'\n'
            {
                [value.as_bytes(), "\n".as_bytes()].concat()
            } else {
                value.into()
            },
            _marker: PhantomData,
        }
    }
}

impl<'a, T> TryFrom<RawTree<T>> for Tree<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: RawTree<T>) -> Result<Self, anyhow::Error> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(T::target_language())
            .expect("Error loading hcl grammar");

        let parsed = parser
            .parse(&value.raw_bytes, None)
            .ok_or(anyhow!("failed to load the code"))?;

        Ok(Tree::new(parsed, value.raw_bytes))
    }
}

impl<T> TryFrom<&str> for Tree<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, anyhow::Error> {
        let r = RawTree::from(value);
        r.try_into()
    }
}

#[allow(unused)]
pub trait TSTreeVisitor<'tree, T>
where
    T: Queryable,
{
    type Output;

    fn walk_leaf_named_node(&self, node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error>;

    fn walk_leaf_unnamed_node(
        &self,
        node: tree_sitter::Node,
    ) -> Result<Self::Output, anyhow::Error>;

    fn walk_intermediate_node(
        &self,
        node: tree_sitter::Node,
    ) -> Result<Self::Output, anyhow::Error> {
        let mut cursor = node.walk();
        let children = node
            .children(&mut cursor)
            .map(|child| self.handle_node(child))
            .collect::<Result<Vec<Self::Output>, anyhow::Error>>()?;

        self.flatten_intermediate_node(node, children)
    }

    fn flatten_intermediate_node(
        &self,
        node: tree_sitter::Node,
        children: Vec<Self::Output>,
    ) -> Result<Self::Output, anyhow::Error>;

    fn walk_ellipsis(&self, node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error>;

    fn walk_ellipsis_metavariable(
        &self,
        node: tree_sitter::Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error>;

    fn walk_metavariable(
        &self,
        node: tree_sitter::Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error>;

    fn handle_node(&self, node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error> {
        match node.kind() {
            SHISHO_NODE_ELLIPSIS => self.walk_ellipsis(node),
            s if s == SHISHO_NODE_ELLIPSIS_METAVARIABLE || s == SHISHO_NODE_METAVARIABLE => {
                let variable_name = node
                    .named_children(&mut node.walk())
                    .find(|child| child.kind() == SHISHO_NODE_METAVARIABLE_NAME)
                    .map(|child| self.value_of(&child))
                    .ok_or(anyhow!(
                        "{} did not have {}",
                        SHISHO_NODE_ELLIPSIS_METAVARIABLE,
                        SHISHO_NODE_METAVARIABLE_NAME
                    ))?;
                if s == SHISHO_NODE_ELLIPSIS_METAVARIABLE {
                    self.walk_ellipsis_metavariable(node, variable_name)
                } else if s == SHISHO_NODE_METAVARIABLE {
                    self.walk_metavariable(node, variable_name)
                } else {
                    panic!("invalid state")
                }
            }
            _ if (self.children_of(node) == 0 || T::is_leaf_like(&node)) => {
                if node.is_named() {
                    self.walk_leaf_named_node(node)
                } else {
                    self.walk_leaf_unnamed_node(node)
                }
            }
            _ => self.walk_intermediate_node(node),
        }
    }

    fn walk(&self, tree: &'tree tree_sitter::Tree) -> Result<Self::Output, anyhow::Error> {
        self.handle_node(tree.root_node())
    }

    fn value_of(&self, node: &tree_sitter::Node) -> &'tree str;

    fn children_of(&self, node: tree_sitter::Node) -> usize {
        node.child_count()
    }
}
