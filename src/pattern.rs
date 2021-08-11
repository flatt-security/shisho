use crate::language::Queryable;
use anyhow::{anyhow, Result};
use std::marker::PhantomData;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum QueryError {
    #[error("ParseError: failed to parse query")]
    ParseError,

    #[error("ParseError: {0}")]
    ConvertError(tree_sitter::QueryError),

    #[error("ParseError: {0}")]
    SyntaxError(String),
}

#[derive(Debug, PartialEq)]
pub struct Pattern<'a, T>
where
    T: Queryable,
{
    raw_bytes: &'a [u8],
    _marker: PhantomData<T>,
}

impl<T> AsRef<[u8]> for Pattern<'_, T>
where
    T: Queryable,
{
    fn as_ref(&self) -> &[u8] {
        self.raw_bytes
    }
}

impl<'a, T> From<&'a str> for Pattern<'a, T>
where
    T: Queryable,
{
    fn from(value: &'a str) -> Self {
        Pattern {
            raw_bytes: value.as_bytes(),
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Pattern<'a, T>
where
    T: Queryable,
{
    pub fn to_tstree(&self) -> Result<tree_sitter::Tree> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(T::query_language())?;

        parser
            .parse(self.raw_bytes, None)
            .ok_or(anyhow!("failed to parse query"))
    }
}
