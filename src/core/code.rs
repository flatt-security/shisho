use crate::core::language::Queryable;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct Code<L>
where
    L: Queryable,
{
    code: String,
    _marker: PhantomData<L>,
}

impl<T> Code<T>
where
    T: Queryable,
{
    pub fn as_str<'a>(&'a self) -> &'a str {
        self.code.as_str()
    }
}

impl<T> AsRef<str> for Code<T>
where
    T: Queryable,
{
    fn as_ref(&self) -> &str {
        self.code.as_str()
    }
}

impl<T, C> From<T> for Code<C>
where
    T: Into<String>,
    C: Queryable,
{
    fn from(code: T) -> Self {
        Self {
            code: code.into(),
            _marker: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct NormalizedSource {
    pub source: Vec<u8>,
}

impl AsRef<[u8]> for NormalizedSource {
    fn as_ref(&self) -> &[u8] {
        &self.source
    }
}

impl From<NormalizedSource> for Vec<u8> {
    fn from(n: NormalizedSource) -> Self {
        n.source
    }
}

impl From<&[u8]> for NormalizedSource {
    fn from(source: &[u8]) -> Self {
        if source.len() != 0 && source[source.len() - 1] != b'\n' {
            Self {
                source: [source, "\n".as_bytes()].concat(),
            }
        } else {
            Self {
                source: source.into(),
            }
        }
    }
}

impl From<String> for NormalizedSource {
    fn from(source: String) -> Self {
        source.as_bytes().into()
    }
}

impl From<&str> for NormalizedSource {
    fn from(source: &str) -> Self {
        source.as_bytes().into()
    }
}
