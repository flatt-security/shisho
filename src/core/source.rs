use anyhow::Result;

use crate::core::{language::Queryable, node::Node};
use std::marker::PhantomData;

use super::{matcher::MatchedItem, rewriter::RewriteOption};

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
    pub fn as_str(&self) -> &str {
        self.code.as_str()
    }
}

impl<T> Code<T>
where
    T: Queryable,
{
    pub fn to_rewritten_form(
        self,
        item: &MatchedItem<'_, Node<'_>>,
        roption: RewriteOption<T>,
    ) -> Result<Self> {
        let current_code = self.as_str().as_bytes();

        let before_snippet = String::from_utf8(current_code[0..item.area.start_byte()].to_vec())?;
        let snippet = roption.into_rewritten_snippet(item)?;
        let after_snippet = String::from_utf8(
            current_code[item.area.end_byte().min(current_code.len())..current_code.len()].to_vec(),
        )?;

        Ok(Code::from(format!(
            "{}{}{}",
            before_snippet, snippet, after_snippet
        )))
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
    with_extra_newline: bool,
}

impl NormalizedSource {
    #[inline]
    pub fn with_extra_newline(&self) -> bool {
        self.with_extra_newline
    }
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
        if !source.is_empty() && source[source.len() - 1] != b'\n' {
            Self {
                source: [source, "\n".as_bytes()].concat(),
                with_extra_newline: true,
            }
        } else {
            Self {
                source: source.into(),
                with_extra_newline: false,
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
