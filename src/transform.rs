use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

use anyhow::Result;
use serde_yaml::to_string;

use crate::{code::Code, language::Queryable, matcher::MatchedItem, pattern::Pattern};

pub struct AutofixPattern<'a, T>
where
    T: Queryable,
{
    pub raw: &'a [u8],
    tree: tree_sitter::Tree,
    _marker: PhantomData<T>,
}

impl<'a, T> AutofixPattern<'a, T>
where
    T: Queryable,
{
    pub fn to_patched_snippet(&self, item: &MatchedItem) -> String {
        "".to_string()
    }
}

impl<'a, T> TryFrom<&'a str> for AutofixPattern<'a, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let tree = Pattern::<T>::new(value).to_tstree()?;
        Ok(AutofixPattern {
            tree,
            raw: value.as_bytes(),
            _marker: PhantomData,
        })
    }
}

pub trait Transformable<T>
where
    T: Queryable,
    Self: Sized,
{
    fn transform<'a, P>(self, p: P, item: MatchedItem) -> Result<Self>
    where
        P: TryInto<AutofixPattern<'a, T>, Error = anyhow::Error>,
        P: 'a,
    {
        let query = p.try_into()?;
        self.transform_with_query(query, item)
    }

    fn transform_with_query(self, query: AutofixPattern<T>, item: MatchedItem) -> Result<Self>;
}

impl<T> Transformable<T> for Code<T>
where
    T: Queryable,
{
    fn transform_with_query(self, query: AutofixPattern<T>, item: MatchedItem) -> Result<Self> {
        let snippet = query.to_patched_snippet(&item);

        let start = item.global.node.start_byte();
        let end = item.global.node.end_byte();

        let raw_code = self.as_str().as_bytes();
        let before = String::from_utf8(raw_code[0..start].to_vec())?;
        let after = String::from_utf8(raw_code[end..raw_code.len()].to_vec())?;

        Ok(Code::new(format!("{}{}{}", before, snippet, after)))
    }
}
