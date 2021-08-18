use crate::{
    language::Queryable,
    matcher::QueryMatcher,
    query::{
        CaptureId, Query, SHISHO_NODE_ELLIPSIS, SHISHO_NODE_ELLIPSIS_METAVARIABLE,
        SHISHO_NODE_METAVARIABLE, SHISHO_NODE_METAVARIABLE_NAME,
    },
};
use anyhow::{anyhow, Result};
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

pub struct Tree<T> {
    raw: Vec<u8>,
    tstree: tree_sitter::Tree,
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

    pub fn to_partial<'node, 'tree>(&'tree self) -> PartialTree<'tree, 'node, T> {
        PartialTree::new(self.tstree.root_node(), self.raw.as_slice())
    }
}

impl<'a, 'tree, T> AsRef<tree_sitter::Tree> for Tree<T> {
    fn as_ref(&self) -> &tree_sitter::Tree {
        &self.tstree
    }
}

impl<'a, 'tree, T> AsRef<[u8]> for Tree<T> {
    fn as_ref(&self) -> &[u8] {
        &self.raw
    }
}

pub struct PartialTree<'tree, 'node, T>
where
    'tree: 'node,
{
    raw: &'tree [u8],
    top: tree_sitter::Node<'node>,
    _marker: PhantomData<T>,
}

impl<'tree, 'node, T> PartialTree<'tree, 'node, T>
where
    T: Queryable,
{
    pub fn new(top: tree_sitter::Node<'node>, raw: &'tree [u8]) -> PartialTree<'tree, 'node, T> {
        PartialTree {
            top,
            raw,
            _marker: PhantomData,
        }
    }

    pub fn matches<'query>(
        &'tree self,
        query: &'query Query<T>,
    ) -> QueryMatcher<'tree, 'node, 'query, T>
    where
        'tree: 'query,
    {
        QueryMatcher::new(self, query)
    }
}

impl<'a, 'tree, 'node, T> AsRef<tree_sitter::Node<'node>> for PartialTree<'tree, 'node, T> {
    fn as_ref(&self) -> &tree_sitter::Node<'node> {
        &self.top
    }
}

impl<'a, 'tree, 'node, T> AsRef<[u8]> for PartialTree<'tree, 'node, T> {
    fn as_ref(&self) -> &[u8] {
        &self.raw
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
        let value = value.as_bytes().clone();
        RawTree {
            raw_bytes: if value[value.len() - 1] != b'\n' {
                [value, "\n".as_bytes()].concat()
            } else {
                value.to_vec()
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

    fn walk_leaf_node(&self, node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error> {
        Err(anyhow!(
            "internal error: walker function for leaf nodes is undefined"
        ))
    }

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

    fn walk_ellipsis(&self, node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error> {
        Err(anyhow!(
            "internal error: walker function for ellipsis operators is undefined"
        ))
    }

    fn walk_ellipsis_metavariable(
        &self,
        node: tree_sitter::Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error> {
        Err(anyhow!(
            "internal error: walker function for ellipsis metavariables is undefined"
        ))
    }

    fn walk_metavariable(
        &self,
        node: tree_sitter::Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error> {
        Err(anyhow!(
            "internal error: walker function for metavariables is undefined"
        ))
    }

    fn handle_node(&self, node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error> {
        match node.kind() {
            SHISHO_NODE_ELLIPSIS => self.walk_ellipsis(node),
            SHISHO_NODE_ELLIPSIS_METAVARIABLE => {
                let vname = self.extract_vname_from_node(&node).ok_or(anyhow!(
                    "{} did not have {}",
                    SHISHO_NODE_ELLIPSIS_METAVARIABLE,
                    SHISHO_NODE_METAVARIABLE_NAME
                ))?;
                self.walk_ellipsis_metavariable(node, vname)
            }
            SHISHO_NODE_METAVARIABLE => {
                let vname = self.extract_vname_from_node(&node).ok_or(anyhow!(
                    "{} did not have {}",
                    SHISHO_NODE_METAVARIABLE,
                    SHISHO_NODE_METAVARIABLE_NAME
                ))?;
                self.walk_metavariable(node, vname)
            }
            _ if self.child_count(node) == 0 || T::is_leaf(&node) => self.walk_leaf_node(node),
            _ => self.walk_intermediate_node(node),
        }
    }

    fn walk(&self, tree: &'tree tree_sitter::Tree) -> Result<Self::Output, anyhow::Error> {
        self.handle_node(tree.root_node())
    }

    fn child_count(&self, node: tree_sitter::Node) -> usize {
        node.child_count()
    }

    fn node_as_str(&self, node: &tree_sitter::Node) -> &'tree str;

    fn extract_vname_from_node(&self, node: &tree_sitter::Node) -> Option<&'tree str> {
        let mut cursor = node.walk();
        let r = node
            .named_children(&mut cursor)
            .find(|child| child.kind() == SHISHO_NODE_METAVARIABLE_NAME)
            .map(|child| self.node_as_str(&child));
        r
    }

    fn node_as_capture_id(&self, node: &tree_sitter::Node) -> CaptureId {
        CaptureId(format!("{}", node.id()))
    }
}
