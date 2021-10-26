use super::{language::Queryable, node::RootNode, source::NormalizedSource};
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
    pub fn to_root_node(&'_ self) -> RootNode<'_> {
        RootNode::from_tstree(&self.tstree, &self.source)
    }

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
            .parse(source.as_ref(), None)
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
