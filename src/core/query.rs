use crate::core::{language::Queryable, pattern::Pattern};
use anyhow::Result;
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

use super::{node::RootNode, source::NormalizedSource};

#[derive(Debug)]
pub struct Query<T>
where
    T: Queryable,
{
    pattern: Pattern<T>,
    _marker: PhantomData<T>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

impl<T> Query<T>
where
    T: Queryable,
{
    pub fn root_node(&'_ self) -> RootNode<'_> {
        self.pattern.root_node()
    }
}

impl<T> TryFrom<&str> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(source: &str) -> Result<Self, anyhow::Error> {
        let source = NormalizedSource::from(source);
        source.try_into()
    }
}

impl<T> TryFrom<NormalizedSource> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(source: NormalizedSource) -> Result<Self, anyhow::Error> {
        let pattern = Pattern::<T>::try_from(source)?;
        Ok(pattern.into())
    }
}

impl<T> From<Pattern<T>> for Query<T>
where
    T: Queryable,
{
    fn from(pattern: Pattern<T>) -> Self {
        Query {
            pattern,
            _marker: PhantomData,
        }
    }
}
