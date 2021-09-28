use super::{code::NormalizedSource, language::Queryable, node::Node};
use anyhow::{anyhow, Result};
use std::{convert::TryFrom, marker::PhantomData};

#[derive(Debug)]
pub struct Pattern<T>
where
    T: Queryable,
{
    pub source: Vec<u8>,

    tstree: tree_sitter::Tree,
    _marker: PhantomData<T>,
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
            source: source.into(),

            tstree,
            _marker: PhantomData,
        })
    }
}
impl<T> Pattern<T>
where
    T: Queryable,
{
    pub fn root_node<'p>(&'p self) -> Box<Node<'p>> {
        Node::from_tsnode(self.tstree.root_node(), &self.source)
    }

    pub fn string_between(&self, start: usize, end: usize) -> Result<String> {
        Ok(String::from_utf8(self.source[start..end].to_vec())?)
    }
}
