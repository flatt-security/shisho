use crate::language::Queryable;
use anyhow::{anyhow, Result};
use std::{array::IntoIter, collections::HashMap, convert::TryFrom, marker::PhantomData};
use thiserror::Error;

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
}

pub trait ToQueryConstraintString {
    fn to_query_constraints(&self) -> Vec<String>;
}

pub type CaptureId = String;
pub type MetavariableTable = HashMap<String, Vec<MetavariableField>>;

impl ToQueryConstraintString for MetavariableTable {
    fn to_query_constraints(&self) -> Vec<String> {
        self.into_iter()
            .filter_map(|(_k, mvs)| {
                let ids: Vec<String> = mvs
                    .into_iter()
                    .map(|field| field.capture_id.clone())
                    .collect();
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
pub struct MetavariableField {
    pub capture_id: CaptureId,
}

impl MetavariableField {
    pub fn new(capture_id: CaptureId) -> Self {
        MetavariableField { capture_id }
    }
}

#[derive(Debug, PartialEq)]
pub struct Metavariable {
    pub id: String,
    pub field: MetavariableField,
}

impl Metavariable {
    pub fn new(id: impl Into<String>, field: MetavariableField) -> Self {
        Metavariable {
            id: id.into(),
            field,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RawQuery<'a, T>
where
    T: Queryable,
{
    raw_bytes: &'a [u8],
    _marker: PhantomData<T>,
}

impl<'a, T> From<&'a str> for RawQuery<'a, T>
where
    T: Queryable,
{
    fn from(value: &'a str) -> Self {
        RawQuery::new(value)
    }
}

impl<'a, T> RawQuery<'a, T>
where
    T: Queryable,
{
    pub fn new(raw_str: &'a str) -> Self {
        RawQuery {
            raw_bytes: raw_str.as_bytes(),
            _marker: PhantomData,
        }
    }
}

impl<'a, T> RawQuery<'a, T>
where
    T: Queryable,
{
    pub fn to_query(&self) -> Result<Query<T>> {
        let TSQueryString {
            query_string,
            metavariables,
            ..
        } = self.to_query_string()?;

        let tsquery = tree_sitter::Query::new(T::target_language(), &query_string)?;

        Ok(Query::new(tsquery, metavariables))
    }

    pub fn to_query_string(&self) -> Result<TSQueryString<T>> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(T::query_language())
            .expect("Error loading hcl-query grammar");

        let query_tree = parser
            .parse(self.raw_bytes, None)
            .ok_or(anyhow!("failed to parse query string"))?;

        let processor = RawQueryProcessor::<T>::new(self.raw_bytes);
        let (child_query_strings, metavariables) =
            processor.handle_nodes(T::extract_query_nodes(&query_tree))?;

        let query = format!(
            "({} {})",
            child_query_strings
                .into_iter()
                .collect::<Vec<String>>()
                .join("."),
            metavariables
                .to_query_constraints()
                .into_iter()
                .collect::<Vec<String>>()
                .join(" ")
        );

        Ok(TSQueryString::new(query, metavariables))
    }
}

#[derive(Debug, PartialEq)]
pub struct RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    raw_bytes: &'a [u8],
    _marker: PhantomData<T>,
}

impl<'a, T> RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    pub fn new(raw_bytes: &'a [u8]) -> Self {
        RawQueryProcessor {
            raw_bytes,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    fn handle_single_node<'b>(&self, node: tree_sitter::Node<'b>) -> Result<TSQueryString<T>>
    where
        'a: 'b,
    {
        match node.kind() {
            "shisho_ellipsis" => Ok(TSQueryString::new("(_)*".into(), MetavariableTable::new())),
            "shisho_metavariable" => {
                // ensure shisho_metavariable has only one child
                let children: Vec<tree_sitter::Node> = {
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor).collect()
                };
                if children.len() != 1 {
                    return Err(QueryError::SyntaxError(format!("shisho_metavariable should have exactly one child (shisho_metavariable_name), but there are {} children", node.child_count()).into()).into());
                }

                // extract child and get shisho_metavariable_name
                let child = children[0];
                if child.kind() != "shisho_metavariable_name" {
                    return Err(QueryError::SyntaxError(format!("shisho_metavariable should have exactly one child (shisho_metavariable_name), but the child was {}", child.kind()).into()).into());
                }
                let variable_name = self.node_as_value(&child).to_string();

                // convert to the tsquery representation with a capture
                let capture_id = format!("{}-{}", variable_name.clone(), node.id());
                // NOTE: into_iter() can't be used here https://github.com/rust-lang/rust/pull/84147
                let metavariables = IntoIter::new([(
                    variable_name,
                    vec![MetavariableField::new(capture_id.clone())],
                )])
                .collect::<MetavariableTable>();

                Ok(TSQueryString::new(
                    format!("(_) @{}", capture_id),
                    metavariables,
                ))
            }
            _ => {
                let children: Vec<tree_sitter::Node> = {
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor).collect()
                };

                if children.len() > 0 {
                    let (child_query_strings, metavariables) = self.handle_nodes(children)?;
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

    fn handle_nodes<'b>(
        &self,
        nodes: Vec<tree_sitter::Node<'b>>,
    ) -> Result<(Vec<String>, MetavariableTable)>
    where
        'a: 'b,
    {
        let mut child_queries = vec![];
        let mut metavariables: MetavariableTable = MetavariableTable::new();

        for node in nodes {
            match self.handle_single_node(node) {
                Ok(TSQueryString {
                    query_string,
                    metavariables: child_metavariables,
                    ..
                }) => {
                    child_queries.push(query_string);
                    for (key, value) in child_metavariables {
                        if let Some(mv) = metavariables.get_mut(&key) {
                            mv.extend(value);
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

impl<T> TryFrom<&str> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        RawQuery::new(value).to_query()
    }
}
