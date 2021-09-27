use crate::core::{language::Queryable, pattern::Pattern};
use anyhow::Result;
use std::{convert::TryFrom, marker::PhantomData};

use super::node::Node;

pub const SHISHO_NODE_METAVARIABLE_NAME: &str = "shisho_metavariable_name";
pub const SHISHO_NODE_METAVARIABLE: &str = "shisho_metavariable";
pub const SHISHO_NODE_ELLIPSIS_METAVARIABLE: &str = "shisho_ellipsis_metavariable";
pub const SHISHO_NODE_ELLIPSIS: &str = "shisho_ellipsis";

#[derive(Debug)]
pub struct Query<'p, T>
where
    T: Queryable,
{
    pattern: Pattern<'p, T>,
    _marker: PhantomData<T>,
}

impl<'p, T> Query<'p, T>
where
    T: Queryable,
{
    pub fn query_nodes(&self) -> Vec<&Box<Node>> {
        T::get_query_nodes(&self.pattern.root)
            .into_iter()
            .filter(|n| !T::is_skippable(n))
            .collect()
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

impl<'p, T> TryFrom<String> for Query<'p, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, anyhow::Error> {
        let p = Pattern::<'p, T>::try_from(value)?;
        Ok(p.into())
    }
}

impl<'p, T> From<Pattern<'p, T>> for Query<'p, T>
where
    T: Queryable,
{
    fn from(pattern: Pattern<'p, T>) -> Self {
        Query {
            pattern,
            _marker: PhantomData,
        }
    }
}
