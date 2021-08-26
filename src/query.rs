use crate::{language::Queryable, pattern::Pattern};
use anyhow::Result;
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

pub const SHISHO_NODE_METAVARIABLE_NAME: &str = "shisho_metavariable_name";
pub const SHISHO_NODE_METAVARIABLE: &str = "shisho_metavariable";
pub const SHISHO_NODE_ELLIPSIS_METAVARIABLE: &str = "shisho_ellipsis_metavariable";
pub const SHISHO_NODE_ELLIPSIS: &str = "shisho_ellipsis";

#[derive(Debug)]
pub struct Query<T>
where
    T: Queryable,
{
    pub(crate) tsquery: tree_sitter::Tree,

    raw: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T> Query<T>
where
    T: Queryable,
{
    pub fn value_of(&self, node: &tree_sitter::Node) -> &str {
        node.utf8_text(self.raw.as_slice()).unwrap()
    }

    pub fn tsnodes(&self) -> Vec<tree_sitter::Node> {
        T::get_query_nodes(&self.tsquery)
            .into_iter()
            .filter(|n| !T::is_skippable(n))
            .collect()
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

impl<T> TryFrom<&str> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, anyhow::Error> {
        let p = Pattern::from(value);
        p.try_into()
    }
}

impl<T> TryFrom<Pattern<T>> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: Pattern<T>) -> Result<Self, anyhow::Error> {
        let query = value.to_tstree()?;
        Ok(Query {
            tsquery: query,
            raw: value.as_ref().to_vec(),
            _marker: PhantomData,
        })
    }
}
