use super::{
    language::Queryable,
    node::{CSTNode, NodeLikeArena, NodeLikeId},
    source::NormalizedSource,
    tree::{RootedTreeLike, TreeLike},
};
use anyhow::{anyhow, Result};
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

#[derive(Debug)]
struct TSPattern<T>
where
    T: Queryable,
{
    tstree: tree_sitter::Tree,
    _marker: PhantomData<T>,
}

impl<T> TryFrom<&NormalizedSource> for TSPattern<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(source: &NormalizedSource) -> Result<Self, anyhow::Error> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(T::query_language())?;

        let tstree = parser
            .parse(source.as_normalized(), None)
            .ok_or(anyhow!("failed to load the code"))?;

        Ok(TSPattern {
            tstree,
            _marker: PhantomData,
        })
    }
}

pub type PatternNode<'tree> = CSTNode<'tree>;
pub type PatternNodeId<'tree> = NodeLikeId<'tree, PatternNode<'tree>>;
pub type PatternNodeArena<'tree> = NodeLikeArena<'tree, PatternNode<'tree>>;

#[derive(Debug)]
pub struct Pattern<'tree, T> {
    pub root: PatternNodeId<'tree>,
    pub source: &'tree NormalizedSource,

    arena: PatternNodeArena<'tree>,
    _marker: PhantomData<T>,
}

impl<'tree, T> Pattern<'tree, T>
where
    T: Queryable,
{
    pub fn new(
        root: PatternNodeId<'tree>,
        arena: PatternNodeArena<'tree>,
        source: &'tree NormalizedSource,
    ) -> Pattern<'tree, T> {
        Pattern {
            root,
            arena,
            source,
            _marker: PhantomData,
        }
    }
}

impl<'tree, T: Queryable> RootedTreeLike<'tree, PatternNode<'tree>> for Pattern<'tree, T> {
    fn root(&'tree self) -> &'tree PatternNode<'tree> {
        self.arena.get(self.root).unwrap()
    }
}

impl<'tree, T: Queryable> TreeLike<'tree, PatternNode<'tree>> for Pattern<'tree, T> {
    fn get(&'tree self, id: PatternNodeId<'tree>) -> Option<&'tree PatternNode<'tree>> {
        self.arena.get(id)
    }
}

impl<'tree, T> From<(TSPattern<T>, &'tree NormalizedSource)> for Pattern<'tree, T>
where
    T: Queryable,
{
    fn from((p, source): (TSPattern<T>, &'tree NormalizedSource)) -> Self {
        let mut arena = NodeLikeArena::new();
        let root = PatternNode::from_tsnode(p.tstree.root_node(), source, &mut arena);
        Pattern::new(root, arena, source)
    }
}

impl<'tree, T> TryFrom<&'tree NormalizedSource> for Pattern<'tree, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &'tree NormalizedSource) -> Result<Self, Self::Error> {
        let p = TSPattern::try_from(value)?;
        Ok((p, value).into())
    }
}
