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
    node::{Node, NodeLike, RootNode},
    query::Query,
    source::NormalizedSource,
};

pub struct Tree<'tree, T> {
    pub(crate) source: NormalizedSource,

    tstree: tree_sitter::Tree,
    _marker: PhantomData<&'tree T>,
}

impl<'tree, T> Tree<'tree, T>
where
    T: Queryable,
{
    pub fn to_root_node(&'_ self) -> RootNode<'_> {
        RootNode::from_tstree(&self.tstree, &self.source)
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
            .parse(nsource.as_normalized(), None)
            .ok_or(anyhow!("failed to load the code"))?;

        Ok(Tree {
            source: nsource,
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
    pub source: &'tree NormalizedSource,
    _marker: PhantomData<T>,
}

impl<'tree, T> NormalizedTree<'tree, T>
where
    T: Queryable,
{
    pub fn new(
        view_root: Node<'tree>,
        source: &'tree NormalizedSource,
    ) -> NormalizedTree<'tree, T> {
        NormalizedTree {
            view_root,
            source,
            _marker: PhantomData,
        }
    }

    pub fn as_ref_treeview(&'tree self) -> RefTreeView<'tree, T, Node<'tree>> {
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

pub struct RefTreeView<'tree, T, N: NodeLike<'tree>> {
    pub view_root: &'tree N,
    _marker: PhantomData<T>,
}

impl<'tree, 'view, T, N: NodeLike<'tree>> RefTreeView<'tree, T, N>
where
    T: Queryable + 'tree,
    'tree: 'view,
{
    pub fn matches<'query>(
        &'view self,
        q: &'query Query<'query, T>,
    ) -> impl Iterator<Item = Result<MatchedItem<'tree, N>>> + 'query + 'view
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

    pub fn traverse(&'view self) -> TreeTreverser<'tree, N> {
        TreeTreverser::new(self.view_root)
    }
}

impl<'tree, T> From<&'tree NormalizedTree<'tree, T>> for RefTreeView<'tree, T, Node<'tree>>
where
    T: Queryable,
{
    fn from(t: &'tree NormalizedTree<'tree, T>) -> Self {
        RefTreeView {
            view_root: &t.view_root,
            _marker: PhantomData,
        }
    }
}

impl<'tree, T, N: NodeLike<'tree>> From<&'tree N> for RefTreeView<'tree, T, N>
where
    T: Queryable,
{
    fn from(t: &'tree N) -> Self {
        RefTreeView {
            view_root: t,
            _marker: PhantomData,
        }
    }
}

pub struct TreeTreverser<'a, N: NodeLike<'a>> {
    queue: VecDeque<(usize, &'a N)>,
}

impl<'a, N: NodeLike<'a>> TreeTreverser<'a, N> {
    #[inline]
    pub fn new(root: &'a N) -> Self {
        Self {
            queue: VecDeque::from(vec![(0, root)]),
        }
    }
}

impl<'tree, N: NodeLike<'tree>> Iterator for TreeTreverser<'tree, N> {
    type Item = (usize, &'tree N);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((depth, node)) = self.queue.pop_front() {
            let children = node.children().into_iter();
            self.queue.extend(children.map(|child| (depth + 1, child)));

            Some((depth, node))
        } else {
            None
        }
    }
}
