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

pub struct NormalizedTree<'tree, T> {
    pub view_root: Node<'tree>,
    pub source: &'tree [u8],
    _marker: PhantomData<T>,
}

impl<'tree, T> NormalizedTree<'tree, T>
where
    T: Queryable,
{
    pub fn new(view_root: Node<'tree>, source: &'tree [u8]) -> NormalizedTree<'tree, T> {
        NormalizedTree {
            view_root,
            source,
            _marker: PhantomData,
        }
    }

    pub fn as_ref_treeview(&'tree self) -> RefTreeView<'tree, T> {
        self.into()
    }
}

impl<'tree, T> From<&'tree Tree<'tree, T>> for NormalizedTree<'tree, T>
where
    T: Queryable,
{
    fn from(t: &'tree Tree<'tree, T>) -> Self {
        NormalizedTree {
            view_root: t.to_root_node().into(),
            source: &t.source,
            _marker: PhantomData,
        }
    }
}

impl<'tree, T> From<Node<'tree>> for NormalizedTree<'tree, T>
where
    T: Queryable,
{
    fn from(t: Node<'tree>) -> Self {
        let source = t.source;
        NormalizedTree {
            view_root: t,
            source,
            _marker: PhantomData,
        }
    }
}

pub struct RefTreeView<'tree, T> {
    pub view_root: &'tree Node<'tree>,
    pub source: &'tree [u8],
    _marker: PhantomData<T>,
}

impl<'tree, 'view, T> RefTreeView<'tree, T>
where
    T: Queryable + 'tree,
    'tree: 'view,
{
    pub fn matches<'query>(
        &'view self,
        q: &'query Query<'query, T>,
    ) -> impl Iterator<Item = Result<MatchedItem<'tree>>> + 'query + 'view
    where
        'tree: 'query,
        'query: 'view,
    {
        TreeMatcher::new(self.traverse(), &q.pattern).filter_map(move |mut x| {
            let captures = match x.satisfies_all(q.constraints) {
                Ok((true, captures)) => captures,
                Ok((false, _)) => return None,
                Err(e) => {
                    return Some(Err(anyhow::anyhow!(
                        "failed to validate a match with constraints: {}",
                        e
                    )))
                }
            };
            x.captures.extend(captures);
            Some(Ok(x))
        })
    }

    pub fn traverse(&'view self) -> TreeTreverser<'tree> {
        TreeTreverser::new(self.view_root)
    }
}

impl<'tree, T> From<&'tree NormalizedTree<'tree, T>> for RefTreeView<'tree, T>
where
    T: Queryable,
{
    fn from(t: &'tree NormalizedTree<'tree, T>) -> Self {
        RefTreeView {
            view_root: &t.view_root,
            source: t.source,
            _marker: PhantomData,
        }
    }
}

impl<'tree, T> From<&'tree Node<'tree>> for RefTreeView<'tree, T>
where
    T: Queryable,
{
    fn from(t: &'tree Node<'tree>) -> Self {
        let source = t.source;
        RefTreeView {
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
