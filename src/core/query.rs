use crate::core::{language::Queryable, pattern::Pattern};
use anyhow::Result;
use std::{convert::TryFrom, marker::PhantomData};

use super::{code::NormalizedSource, node::Node};

pub const SHISHO_NODE_METAVARIABLE_NAME: &str = "shisho_metavariable_name";
pub const SHISHO_NODE_METAVARIABLE: &str = "shisho_metavariable";
pub const SHISHO_NODE_ELLIPSIS_METAVARIABLE: &str = "shisho_ellipsis_metavariable";
pub const SHISHO_NODE_ELLIPSIS: &str = "shisho_ellipsis";

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
    pub fn root_node<'p>(&'p self) -> Box<Node<'p>> {
        self.pattern.root_node()
    }
}

impl<T> TryFrom<String> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, anyhow::Error> {
        let source = NormalizedSource::from(value);
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