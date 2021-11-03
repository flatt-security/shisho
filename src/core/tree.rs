use crate::core::language::Queryable;
use anyhow::{anyhow, Result};
use std::{collections::VecDeque, convert::TryFrom, marker::PhantomData};

use super::{
    node::{CSTNode, NodeLike, NodeLikeArena, NodeLikeId, NodeLikeRefWithId},
    source::NormalizedSource,
};

struct TSTree<'tree, T> {
    tstree: tree_sitter::Tree,
    _marker: PhantomData<&'tree T>,
}

impl<'tree, T> TryFrom<&'tree NormalizedSource> for TSTree<'tree, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(nsource: &'tree NormalizedSource) -> Result<Self, anyhow::Error> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(T::target_language())
            .expect("Error loading hcl grammar");

        let tstree = parser
            .parse(nsource.as_normalized(), None)
            .ok_or(anyhow!("failed to load the code"))?;

        Ok(TSTree {
            tstree,
            _marker: PhantomData,
        })
    }
}

pub struct Tree<'tree, T, N: NodeLike<'tree>> {
    pub root_id: NodeLikeId<'tree, N>,
    pub source: &'tree NormalizedSource,

    arena: NodeLikeArena<'tree, N>,
    _marker: PhantomData<&'tree T>,
}

pub type CST<'tree, T> = Tree<'tree, T, CSTNode<'tree>>;

impl<'tree, T> From<(TSTree<'tree, T>, &'tree NormalizedSource)> for Tree<'tree, T, CSTNode<'tree>>
where
    T: Queryable,
{
    fn from((t, source): (TSTree<'tree, T>, &'tree NormalizedSource)) -> Self {
        let mut arena = NodeLikeArena::new();
        let root_id = CSTNode::from_tsnode(t.tstree.root_node(), source, &mut arena);
        Tree {
            root_id,
            source,
            arena,
            _marker: PhantomData,
        }
    }
}

impl<'tree, T> TryFrom<&'tree NormalizedSource> for Tree<'tree, T, CSTNode<'tree>>
where
    T: Queryable + 'tree,
{
    type Error = anyhow::Error;

    fn try_from(nsource: &'tree NormalizedSource) -> Result<Self, anyhow::Error> {
        let r = TSTree::try_from(nsource)?;
        Ok((r, nsource).into())
    }
}

pub trait Traversable<'tree, T: Queryable, N: NodeLike<'tree>> {
    fn traverse(&'tree self, from: NodeLikeId<'tree, N>) -> TreeTreverser<'tree, T, N>;
}

pub trait RootedTreeLike<'tree, N: NodeLike<'tree>> {
    fn root(&'tree self) -> Option<&'tree N>;
    fn get(&'tree self, id: NodeLikeId<'tree, N>) -> Option<&'tree N>;
}

impl<'tree, T: Queryable + 'tree, N: NodeLike<'tree> + 'tree> Traversable<'tree, T, N>
    for TreeView<'tree, T, N>
{
    fn traverse(&'tree self, id: NodeLikeId<'tree, N>) -> TreeTreverser<'tree, T, N> {
        TreeTreverser::new(self.get_with_id(id).unwrap(), self)
    }
}

pub struct TreeTreverser<'a, T: Queryable, N: NodeLike<'a>> {
    tview: &'a TreeView<'a, T, N>,
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

pub struct TreeView<'tree, T, N: NodeLike<'tree>> {
    pub root_id: NodeLikeId<'tree, N>,
    pub source: &'tree NormalizedSource,

    arena: &'tree NodeLikeArena<'tree, N>,
    _marker: PhantomData<T>,
}

pub type CSTView<'tree, T> = TreeView<'tree, T, CSTNode<'tree>>;

impl<'tree, T, N> From<&'tree Tree<'tree, T, N>> for TreeView<'tree, T, N>
where
    T: Queryable + 'tree,
    N: NodeLike<'tree>,
{
    fn from(t: &'tree Tree<'tree, T, N>) -> Self {
        TreeView::new(t.root_id.clone(), &t.arena, &t.source)
    }
}

impl<'tree, T: Queryable, N: NodeLike<'tree>> RootedTreeLike<'tree, N> for TreeView<'tree, T, N> {
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
        arena: &'tree NodeLikeArena<'tree, N>,
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
