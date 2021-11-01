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
    node::{Node, NodeLike, NodeLikeArena, NodeLikeId},
    query::Query,
    source::NormalizedSource,
};

pub struct Tree<'tree, T> {
    pub(crate) source: NormalizedSource,

    tstree: tree_sitter::Tree,
    _marker: PhantomData<&'tree T>,
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

pub struct TreeView<'tree, T, N: NodeLike<'tree>> {
    pub root: NodeLikeId<'tree, N>,
    pub arena: NodeLikeArena<'tree, N>,
    pub source: &'tree NormalizedSource,
    _marker: PhantomData<T>,
}

impl<'tree, T, N: NodeLike<'tree>> TreeView<'tree, T, N>
where
    T: Queryable,
{
    pub fn new(
        root: NodeLikeId<'tree, N>,
        arena: NodeLikeArena<'tree, N>,
        source: &'tree NormalizedSource,
    ) -> TreeView<'tree, T, N> {
        TreeView {
            root,
            arena,
            source,
            _marker: PhantomData,
        }
    }

    pub fn get(&'tree self, id: NodeLikeId<'tree, N>) -> Option<&'tree N> {
        self.arena.get(id)
    }
}

impl<'tree, T> From<&'tree Tree<'tree, T>> for TreeView<'tree, T, Node<'tree>>
where
    T: Queryable,
{
    fn from(t: &'tree Tree<'tree, T>) -> Self {
        let mut arena = NodeLikeArena::new();
        let root = Node::from_tsnode(t.tstree.root_node(), &t.source, &mut arena);
        TreeView::new(root, arena, &t.source)
    }
}

impl<'tree, T, N: NodeLike<'tree>> TreeView<'tree, T, N>
where
    T: Queryable + 'tree,
    N: 'tree,
{
    pub fn matches<'query>(
        &'tree self,
        q: &'query Query<'query, T>,
    ) -> impl Iterator<Item = Result<MatchedItem<'tree, N>>> + 'query
    where
        'tree: 'query,
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

    pub fn traverse(&'tree self) -> TreeTreverser<'tree, N> {
        TreeTreverser::new(self.get(self.root).unwrap(), &self.arena)
    }
}

pub struct TreeTreverser<'a, N: NodeLike<'a>> {
    queue: VecDeque<(usize, &'a N)>,
    arena: &'a NodeLikeArena<'a, N>,
}

impl<'a, N: NodeLike<'a>> TreeTreverser<'a, N> {
    #[inline]
    pub fn new(root: &'a N, arena: &'a NodeLikeArena<'a, N>) -> Self {
        Self {
            queue: VecDeque::from(vec![(0, root)]),
            arena,
        }
    }
}

impl<'tree, N: NodeLike<'tree>> Iterator for TreeTreverser<'tree, N> {
    type Item = (usize, &'tree N);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((depth, node)) = self.queue.pop_front() {
            let children = node.children(self.arena).into_iter();
            self.queue.extend(children.map(|child| (depth + 1, child)));

            Some((depth, node))
        } else {
            None
        }
    }
}
