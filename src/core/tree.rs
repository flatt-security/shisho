use crate::core::{
    language::Queryable,
    matcher::{MatchedItem, TreeMatcher},
};
use anyhow::{anyhow, Result};
use std::{
    collections::VecDeque,
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

use super::{
    node::{Node, RootNode},
    query::Query,
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
        qc: &'query Query<'query, T>,
    ) -> impl Iterator<Item = Result<MatchedItem<'tree>>> + 'query
    where
        'tree: 'query,
    {
        TreeMatcher::new(self, &qc.query).filter_map(move |x| {
            // TODO (y0n3uchy): is this unwrap_or okay? do we need to emit errors?
            match x.satisfies_all(&qc.constraints) {
                Ok(true) => Some(Ok(x)),
                Ok(false) => None,
                Err(e) => Some(Err(anyhow::anyhow!(
                    "failed to validate a match with constraints: {}",
                    e
                ))),
            }
        })
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
