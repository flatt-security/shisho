use crate::{language::Queryable, matcher::QueryMatcher, query::Query};
use anyhow::Result;
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TreeError {
    #[error("ParseError: failed to parse query")]
    ParseError,

    #[error("ParseError: {0}")]
    ConvertError(tree_sitter::QueryError),
}

pub struct Tree<'a, T> {
    pub raw: &'a [u8],

    tstree: tree_sitter::Tree,
    _marker: PhantomData<T>,
}

impl<'tree, T> Tree<'tree, T>
where
    T: Queryable,
{
    pub fn new(tree: tree_sitter::Tree, raw: &'tree [u8]) -> Tree<'tree, T> {
        Tree {
            tstree: tree,
            raw,
            _marker: PhantomData,
        }
    }

    pub fn to_partial<'node>(&'tree self) -> PartialTree<'tree, 'node, T> {
        PartialTree::new(self.tstree.root_node(), self.raw)
    }

    pub(crate) fn as_ts_tree<'a>(&'a self) -> &'a tree_sitter::Tree {
        &self.tstree
    }
}

pub struct PartialTree<'tree, 'node, T>
where
    'tree: 'node,
{
    pub raw: &'tree [u8],

    top: &'tree tree_sitter::Node<'tree>,
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

    pub(crate) fn as_ts_node<'a>(&'a self) -> &'a tree_sitter::Node {
        &self.top
    }
}

#[derive(Debug, PartialEq)]
pub struct RawTree<'a, T>
where
    T: Queryable,
{
    raw_bytes: &'a [u8],
    _marker: PhantomData<T>,
}

impl<'a, T> From<&'a str> for RawTree<'a, T>
where
    T: Queryable,
{
    fn from(value: &'a str) -> Self {
        RawTree {
            raw_bytes: value.as_bytes(),
            _marker: PhantomData,
        }
    }
}

impl<'a, T> TryFrom<RawTree<'a, T>> for Tree<'a, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: RawTree<'a, T>) -> Result<Self, Self::Error> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(T::target_language())
            .expect("Error loading hcl grammar");

        Ok(Tree::new(
            parser.parse(value.raw_bytes, None).unwrap(),
            value.raw_bytes,
        ))
    }
}

impl<'a, T> TryFrom<&'a str> for Tree<'a, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let r = RawTree::from(value);
        r.try_into()
    }
}
