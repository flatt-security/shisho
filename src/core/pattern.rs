use super::{language::Queryable, node::Node};
use anyhow::{anyhow, Result};
use std::{convert::TryFrom, marker::PhantomData};

#[derive(Debug, PartialEq)]
pub struct Pattern<'tree, T>
where
    T: Queryable,
{
    pub root: Box<Node<'tree>>,
    pub source: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<'tree, T> TryFrom<String> for Pattern<'tree, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, anyhow::Error> {
        let source = if value.as_bytes().len() != 0
            && value.as_bytes()[value.as_bytes().len() - 1] != b'\n'
        {
            [value.as_bytes(), "\n".as_bytes()].concat()
        } else {
            value.into()
        };

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(T::query_language())?;

        let parsed = parser
            .parse(&source, None)
            .ok_or(anyhow!("failed to load the code"))?;

        Ok(Pattern {
            root: Node::from_tsnode(parsed.root_node(), source.as_slice()),
            source,
            _marker: PhantomData,
        })
    }
}
impl<'p, T> Pattern<'p, T>
where
    T: Queryable,
{
    pub fn string_between(&self, start: usize, end: usize) -> Result<String> {
        Ok(String::from_utf8(self.source[start..end].to_vec())?)
    }
}
