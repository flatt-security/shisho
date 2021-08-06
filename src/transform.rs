use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

use anyhow::Result;

use crate::{code::Code, language::Queryable, matcher::MatchedItem, pattern::Pattern};

pub struct AutofixPattern<T>
where
    T: Queryable,
{
    tree: tree_sitter::Tree,
    _marker: PhantomData<T>,
}

impl<T> TryFrom<&str> for AutofixPattern<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let tree = Pattern::<T>::new(value).to_tstree()?;
        Ok(AutofixPattern {
            tree: tree,
            _marker: PhantomData,
        })
    }
}

pub trait Transformable<T>
where
    T: Queryable,
    Self: Sized,
{
    fn transform<P>(self, p: P, item: MatchedItem) -> Result<Self>
    where
        P: TryInto<AutofixPattern<T>, Error = anyhow::Error>,
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
        Ok(self)
    }
}
