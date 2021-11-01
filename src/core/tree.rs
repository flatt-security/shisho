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
    node::{Node, NodeLike, NodeLikeArena, NodeLikeId, NodeLikeRefWithId, NodeRefWithId},
    query::Query,
    source::NormalizedSource,
    view::NodeLikeView,
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
    pub root_id: NodeLikeId<'tree, N>,
    pub source: &'tree NormalizedSource,

    arena: NodeLikeArena<'tree, N>,
    _marker: PhantomData<T>,
}

impl<'tree, T: Queryable, N: NodeLike<'tree>> NodeLikeView<'tree, N> for TreeView<'tree, T, N> {
    fn root(&'tree self) -> Option<&'tree N> {
        self.arena.get(self.root_id)
    }

    fn get(&'tree self, id: NodeLikeId<'tree, N>) -> Option<&'tree N> {
        self.arena.get(id)
    }
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
            root_id: root,
            arena,
            source,
            _marker: PhantomData,
        }
    }

    pub fn get(&'tree self, id: NodeLikeId<'tree, N>) -> Option<&'tree N> {
        self.arena.get(id)
    }

    pub fn get_with_id(
        &'tree self,
        id: NodeLikeId<'tree, N>,
    ) -> Option<NodeLikeRefWithId<'tree, N>> {
        self.arena
            .get(id)
            .map(|x| NodeLikeRefWithId { id, node: x })
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
        self.matches_from(self.root_id, q)
    }

    pub fn matches_from<'query>(
        &'tree self,
        id: NodeLikeId<'tree, N>,
        q: &'query Query<'query, T>,
    ) -> impl Iterator<Item = Result<MatchedItem<'tree, N>>> + 'query
    where
        'tree: 'query,
    {
        TreeMatcher::new(self, id, &q.pattern).filter_map(move |x| {
            match x.filter(self, q.constraints) {
                Ok(None) => None,
                Ok(Some(x)) => Some(Ok(x)),
                Err(e) => Some(Err(e)),
            }
        })
    }
}

pub trait Traversable<'tree, T: Queryable, N: NodeLike<'tree>> {
    fn traverse(self, from: NodeLikeId<'tree, N>) -> TreeTreverser<'tree, T, N>;
}

impl<'tree, T: Queryable + 'tree, N: NodeLike<'tree> + 'tree> Traversable<'tree, T, N>
    for &'tree TreeView<'tree, T, N>
{
    fn traverse(self, id: NodeLikeId<'tree, N>) -> TreeTreverser<'tree, T, N> {
        TreeTreverser::new(self.get_with_id(id).unwrap(), self)
    }
}

pub struct TreeTreverser<'a, T: Queryable, N: NodeLike<'a>> {
    pub tview: &'a TreeView<'a, T, N>,
    queue: VecDeque<(usize, NodeLikeRefWithId<'a, N>)>,
}

impl<'a, T: Queryable, N: NodeLike<'a>> TreeTreverser<'a, T, N> {
    #[inline]
    pub fn new(from: NodeLikeRefWithId<'a, N>, tview: &'a TreeView<'a, T, N>) -> Self {
        Self {
            queue: VecDeque::from(vec![(0, from)]),
            tview,
        }
    }
}

impl<'tree, T: Queryable, N: NodeLike<'tree>> Iterator for TreeTreverser<'tree, T, N> {
    type Item = (usize, NodeLikeRefWithId<'tree, N>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((depth, node)) = self.queue.pop_front() {
            let children = node.children(self.tview).into_iter();
            self.queue.extend(children.map(|child| (depth + 1, child)));

            Some((depth, node))
        } else {
            None
        }
    }
}
