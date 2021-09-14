use crate::core::language::Queryable;
use anyhow::{anyhow, Result};
use std::marker::PhantomData;

#[derive(Debug, PartialEq)]
pub struct Pattern<T>
where
    T: Queryable,
{
    raw_bytes: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T> AsRef<[u8]> for Pattern<T>
where
    T: Queryable,
{
    fn as_ref(&self) -> &[u8] {
        &self.raw_bytes
    }
}

impl<'a, T> From<&'a str> for Pattern<T>
where
    T: Queryable,
{
    fn from(value: &'a str) -> Self {
        let value = value.to_string();
        Pattern {
            raw_bytes: if value.as_bytes()[value.as_bytes().len() - 1] != b'\n' {
                [value.as_bytes(), "\n".as_bytes()].concat()
            } else {
                value.into()
            },
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Pattern<T>
where
    T: Queryable,
{
    pub fn to_tstree(&self) -> Result<tree_sitter::Tree> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(T::query_language())?;

        parser
            .parse(&self.raw_bytes, None)
            .ok_or(anyhow!("failed to parse query"))
    }
}
