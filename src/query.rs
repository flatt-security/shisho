use crate::language::Queryable;
use anyhow::{anyhow, Result};
use std::{convert::TryFrom, marker::PhantomData};
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
    pub metavariables: Vec<Metavariable>,

    query: tree_sitter::Query,
    _marker: PhantomData<T>,
}

impl<T> Query<T>
where
    T: Queryable,
{
    pub fn new(query: tree_sitter::Query, metavariables: Vec<Metavariable>) -> Self {
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

#[derive(Debug, PartialEq)]
pub struct Metavariable {
    pub id: String,
}

impl Metavariable {
    pub fn new(id: impl Into<String>) -> Metavariable {
        Metavariable { id: id.into() }
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

        let query_nodes = T::extract_query_nodes(&query_tree);
        if query_nodes.len() == 1 {
            self.node_to_query_string(query_nodes[0])
        } else {
            self.nodes_to_query_string(None, query_nodes)
        }
    }

    fn node_to_query_string<'b>(&self, node: tree_sitter::Node<'b>) -> Result<TSQueryString<T>>
    where
        'a: 'b,
    {
        match node.kind() {
            "shisho_ellipsis" => Ok(TSQueryString::new("(_)*".into(), vec![])),
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
                let metavariable = Metavariable::new(variable_name.clone());
                Ok(TSQueryString::new(
                    format!("(_) @{}", variable_name),
                    vec![metavariable],
                ))
            }
            _ => {
                let children: Vec<tree_sitter::Node> = {
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor).collect()
                };

                if children.len() > 0 {
                    self.nodes_to_query_string(Some(node), children)
                } else {
                    Ok(TSQueryString::new(
                        format!(
                            r#"(({}) @{} (#eq? @{} "{}"))"#,
                            node.kind(),
                            node.id(),
                            node.id(),
                            self.node_as_value(&node).replace("\"", "\\\"")
                        ),
                        vec![],
                    ))
                }
            }
        }
    }

    fn nodes_to_query_string<'b>(
        &self,
        parent: Option<tree_sitter::Node<'b>>,
        nodes: Vec<tree_sitter::Node<'b>>,
    ) -> Result<TSQueryString<T>>
    where
        'a: 'b,
    {
        let mut child_queries = vec![];
        let mut child_metavariables = vec![];

        for node in nodes {
            match self.node_to_query_string(node) {
                Ok(TSQueryString {
                    query_string,
                    metavariables,
                    ..
                }) => {
                    child_queries.push(query_string);
                    child_metavariables.extend(metavariables);
                }
                Err(e) => return Err(e),
            }
        }

        if let Some(node) = parent {
            Ok(TSQueryString::new(
                format!(
                    r#"({} . {} .)"#,
                    node.kind(),
                    child_queries.into_iter().collect::<Vec<String>>().join("."),
                ),
                child_metavariables,
            ))
        } else {
            Ok(TSQueryString::new(
                format!(
                    r#"({})"#,
                    child_queries.into_iter().collect::<Vec<String>>().join("."),
                ),
                child_metavariables,
            ))
        }
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
    pub metavariables: Vec<Metavariable>,
    pub query_string: String,

    _marker: PhantomData<T>,
}

impl<T> TSQueryString<T>
where
    T: Queryable,
{
    pub fn new(query_string: String, metavariables: Vec<Metavariable>) -> Self {
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
