use crate::{language::Queryable, pattern::Pattern};
use anyhow::Result;
use std::{
    array::IntoIter,
    collections::HashMap,
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};
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

    pub fn get_cid_mvid_map(&self) -> HashMap<CaptureId, MetavariableId> {
        self.metavariables
            .iter()
            .map(|(k, v)| v.capture_ids.iter().map(move |id| (id.clone(), k.clone())))
            .flatten()
            .collect::<HashMap<CaptureId, MetavariableId>>()
    }

    pub(crate) fn ts_query<'a>(&'a self) -> &'a tree_sitter::Query {
        &self.query
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct CaptureId(pub String);

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

pub type MetavariableTable = HashMap<MetavariableId, MetavariableField>;

#[derive(Debug, PartialEq)]
pub struct MetavariableField {
    pub capture_ids: Vec<CaptureId>,
}

impl MetavariableField {
    pub fn new(capture_ids: Vec<CaptureId>) -> Self {
        MetavariableField { capture_ids }
    }
}

impl<T> TryFrom<&str> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let p = Pattern::from(value);
        p.try_into()
    }
}

impl<T> TryFrom<Pattern<'_, T>> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: Pattern<'_, T>) -> Result<Self, Self::Error> {
        let tsq = TSQueryString::try_from(value)?;

        let tsquery = tree_sitter::Query::new(T::target_language(), &tsq.query_string)?;

        Ok(Query::new(tsquery, tsq.metavariables))
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

impl<T> TryFrom<&str> for TSQueryString<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let p = Pattern::from(value);
        p.try_into()
    }
}

impl<T> TryFrom<Pattern<'_, T>> for TSQueryString<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: Pattern<'_, T>) -> Result<Self, Self::Error> {
        let query_tree = value.to_tstree()?;

        let processor = RawQueryProcessor::<T>::from(value.as_ref());
        let (child_query_strings, metavariables) =
            processor.convert_nodes(T::extract_query_nodes(&query_tree))?;

        let query_string = format!(
            "({} {})",
            child_query_strings
                .into_iter()
                .enumerate()
                .map(|(index, query)| format!(" {} @{}{} ", query, TOP_CAPTURE_ID_PREFIX, index))
                .collect::<Vec<String>>()
                .join("."),
            metavariables
                .to_query_constraints()
                .into_iter()
                .collect::<Vec<String>>()
                .join(" "),
        );

        Ok(TSQueryString {
            query_string,
            metavariables,
            _marker: PhantomData,
        })
    }
}

pub trait ToQueryConstraintString {
    fn to_query_constraints(&self) -> Vec<String>;
}

impl ToQueryConstraintString for MetavariableTable {
    fn to_query_constraints(&self) -> Vec<String> {
        self.into_iter()
            .filter_map(|(_k, mvs)| {
                let ids: Vec<String> = mvs.capture_ids.iter().map(|id| id.0.clone()).collect();
                if ids.len() <= 1 {
                    None
                } else {
                    let first_capture_id = ids[0].clone();
                    Some(
                        ids.into_iter()
                            .skip(1)
                            .map(|id| format!("(#eq? @{} @{})", first_capture_id, id))
                            .collect::<Vec<String>>()
                            .join(" "),
                    )
                }
            })
            .collect()
    }
}

#[derive(Debug, PartialEq)]
struct RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    raw_bytes: &'a [u8],
    _marker: PhantomData<T>,
}

impl<'a, T> From<&'a [u8]> for RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    fn from(value: &'a [u8]) -> Self {
        RawQueryProcessor {
            raw_bytes: value,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    fn convert_node<'b>(&self, node: tree_sitter::Node<'b>) -> Result<TSQueryString<T>>
    where
        'a: 'b,
    {
        match node.kind() {
            "shisho_ellipsis" => Ok(TSQueryString::new(
                " ((_) _?)*  . ((_) _?)? ".into(),
                MetavariableTable::new(),
            )),
            "shisho_ellipsis_metavariable" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    // TODO (y0n3uchy): refactor these lines by introducing appropriate abstraction?
                    if child.kind() == "shisho_metavariable_name" {
                        let variable_name = self.node_as_value(&child).to_string();

                        // convert to the tsquery representation with a capture
                        let capture_id = format!("{}-{}", variable_name.clone(), node.id());
                        // NOTE: into_iter() can't be used here https://github.com/rust-lang/rust/pull/84147
                        let metavariables = IntoIter::new([(
                            MetavariableId(variable_name),
                            MetavariableField::new(vec![CaptureId(capture_id.clone())]),
                        )])
                        .collect::<MetavariableTable>();

                        return Ok(TSQueryString::new(
                            format!(" ((_) _?)* @{} . ((_) _?)? @{} ", capture_id, capture_id),
                            metavariables,
                        ));
                    }
                }
                return Err(QueryError::SyntaxError(format!("shisho_metavariable should have exactly one child (shisho_metavariable_name), but there are {} children", node.child_count()).into()).into());
            }
            "shisho_metavariable" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    // TODO (y0n3uchy): refactor these lines by introducing appropriate abstraction?
                    if child.kind() == "shisho_metavariable_name" {
                        let variable_name = self.node_as_value(&child).to_string();

                        // convert to the tsquery representation with a capture
                        let capture_id = format!("{}-{}", variable_name.clone(), node.id());
                        // NOTE: into_iter() can't be used here https://github.com/rust-lang/rust/pull/84147
                        let metavariables = IntoIter::new([(
                            MetavariableId(variable_name),
                            MetavariableField::new(vec![CaptureId(capture_id.clone())]),
                        )])
                        .collect::<MetavariableTable>();

                        return Ok(TSQueryString::new(
                            format!("((_) @{})", capture_id),
                            metavariables,
                        ));
                    }
                }
                return Err(QueryError::SyntaxError(format!("shisho_metavariable should have exactly one child (shisho_metavariable_name), but there are {} children", node.child_count()).into()).into());
            }
            _ => {
                let children: Vec<tree_sitter::Node> = {
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor).collect()
                };

                if children.len() > 0 {
                    let (child_query_strings, metavariables) = self.convert_nodes(children)?;
                    Ok(TSQueryString::new(
                        format!(
                            r#"({} . {} .)"#,
                            node.kind(),
                            child_query_strings
                                .into_iter()
                                .collect::<Vec<String>>()
                                .join("."),
                        ),
                        metavariables,
                    ))
                } else {
                    Ok(TSQueryString::new(
                        format!(
                            r#"(({}) @{} (#eq? @{} "{}"))"#,
                            node.kind(),
                            node.id(),
                            node.id(),
                            self.node_as_value(&node).replace("\"", "\\\"")
                        ),
                        MetavariableTable::new(),
                    ))
                }
            }
        }
    }

    fn convert_nodes<'b>(
        &self,
        nodes: Vec<tree_sitter::Node<'b>>,
    ) -> Result<(Vec<String>, MetavariableTable)>
    where
        'a: 'b,
    {
        let mut child_queries = vec![];
        let mut metavariables: MetavariableTable = MetavariableTable::new();

        for node in nodes {
            match self.convert_node(node) {
                Ok(TSQueryString {
                    query_string,
                    metavariables: child_metavariables,
                    ..
                }) => {
                    child_queries.push(query_string);
                    for (key, value) in child_metavariables {
                        if let Some(mv) = metavariables.get_mut(&key) {
                            mv.capture_ids.extend(value.capture_ids);
                        } else {
                            metavariables.insert(key, value);
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Ok((child_queries, metavariables))
    }

    fn node_as_value<'b>(&self, node: &'b tree_sitter::Node) -> &'a str
    where
        'a: 'b,
    {
        // NOTE: this unwrap won't fail.
        node.utf8_text(self.raw_bytes).unwrap()
    }
}
