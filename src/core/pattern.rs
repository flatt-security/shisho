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
    pub source: Vec<u8>,
    with_extra_newline: bool,

    tstree: tree_sitter::Tree,
    _marker: PhantomData<T>,
}

impl<T> Pattern<T>
where
    T: Queryable,
{
    pub fn to_root_node(&'_ self) -> RootNode<'_> {
        RootNode::from_tstree(&self.tstree, &self.source, self.with_extra_newline)
    }

    #[inline]
    pub fn string_between(&self, start: usize, end: usize) -> Result<String> {
        let start = if self.source.len() == start && self.with_extra_newline {
            start - 1
        } else {
            start
        };
        let end = if self.source.len() == end && self.with_extra_newline {
            end - 1
        } else {
            end
        };
        Ok(String::from_utf8(self.source[start..end].to_vec())?)
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

        let with_extra_newline = source.with_extra_newline();
        Ok(Pattern {
            source: source.into(),
            with_extra_newline,

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
