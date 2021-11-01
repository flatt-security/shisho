use super::{
    language::Queryable,
    node::{Node, NodeLike, NodeLikeArena, NodeLikeId},
    source::NormalizedSource,
};
use anyhow::{anyhow, Result};
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

#[derive(Debug)]
pub struct Pattern<T>
where
    T: Queryable,
{
    pub(crate) source: NormalizedSource,

    tstree: tree_sitter::Tree,
    _marker: PhantomData<T>,
}

impl<T> Pattern<T>
where
    T: Queryable,
{
    #[inline]
    pub fn as_str_between(&self, start: usize, end: usize) -> Result<&str> {
        self.source.as_str_between(start, end)
    }
}

impl<T> TryFrom<NormalizedSource> for Pattern<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(source: NormalizedSource) -> Result<Self, anyhow::Error> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(T::query_language())?;

        let tstree = parser
            .parse(source.as_normalized(), None)
            .ok_or(anyhow!("failed to load the code"))?;

        Ok(Pattern {
            source,
            tstree,
            _marker: PhantomData,
        })
    }
}

impl<T> TryFrom<&str> for Pattern<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(source: &str) -> Result<Self, anyhow::Error> {
        let source = NormalizedSource::from(source);
        source.try_into()
    }
}

pub type PatternNode<'tree> = Node<'tree>;
pub type PatternNodeId<'tree> = NodeLikeId<'tree, Node<'tree>>;
pub type PatternNodeArena<'tree> = NodeLikeArena<'tree, Node<'tree>>;

#[derive(Debug)]
pub struct PatternView<'tree, T> {
    pub root: PatternNodeId<'tree>,
    pub arena: PatternNodeArena<'tree>,
    pub source: &'tree NormalizedSource,
    _marker: PhantomData<T>,
}

impl<'tree, T> PatternView<'tree, T>
where
    T: Queryable,
{
    pub fn new(
        root: PatternNodeId<'tree>,
        arena: PatternNodeArena<'tree>,
        source: &'tree NormalizedSource,
    ) -> PatternView<'tree, T> {
        PatternView {
            root,
            arena,
            source,
            _marker: PhantomData,
        }
    }

    pub fn get(&'tree self, id: PatternNodeId<'tree>) -> Option<&'tree PatternNode<'tree>> {
        self.arena.get(id)
    }
}

impl<'tree, T> From<&'tree Pattern<T>> for PatternView<'tree, T>
where
    T: Queryable,
{
    fn from(p: &'tree Pattern<T>) -> Self {
        let mut arena = NodeLikeArena::new();
        let root = Node::from_tsnode(p.tstree.root_node(), &p.source, &mut arena);
        PatternView::new(root, arena, &p.source)
    }
}
