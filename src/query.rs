use crate::{language::Queryable, pattern::Pattern};
use std::{collections::HashMap, convert::TryFrom, marker::PhantomData};
use thiserror::Error;

pub const TOP_CAPTURE_ID_PREFIX: &str = "TOP-";

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
pub struct Query<T>
where
    T: Queryable,
{
    pub metavariables: MetavariableTable,

    query: tree_sitter::Query,
    _marker: PhantomData<T>,
}

impl<T> Query<T>
where
    T: Queryable,
{
    pub fn new(query: tree_sitter::Query, metavariables: MetavariableTable) -> Self {
        Query {
            query,
            metavariables,
            _marker: PhantomData,
        }
    }

    pub fn ts_query<'a>(&'a self) -> &'a tree_sitter::Query {
        &self.query
    }

    pub fn get_cid_mvid_map(&self) -> HashMap<CaptureId, MetavariableId> {
        self.metavariables
            .iter()
            .map(|(k, v)| {
                v.into_iter()
                    .map(move |field| (field.capture_id.clone(), k.clone()))
            })
            .flatten()
            .collect::<HashMap<CaptureId, MetavariableId>>()
    }
}

#[derive(Debug, PartialEq)]
pub struct TSQueryString<T>
where
    T: Queryable,
{
    pub metavariables: MetavariableTable,
    pub query_string: String,

    _marker: PhantomData<T>,
}

impl<T> TSQueryString<T>
where
    T: Queryable,
{
    pub fn new(query_string: String, metavariables: MetavariableTable) -> Self {
        TSQueryString {
            query_string,
            metavariables,
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct CaptureId(pub String);

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

pub type MetavariableTable = HashMap<MetavariableId, Vec<MetavariableField>>;

#[derive(Debug, PartialEq)]
pub struct MetavariableField {
    pub capture_id: CaptureId,
}

impl MetavariableField {
    pub fn new(capture_id: CaptureId) -> Self {
        MetavariableField { capture_id }
    }
}

impl<T> TryFrom<&str> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Pattern::new(value).to_query()
    }
}
