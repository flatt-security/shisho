use anyhow::{anyhow, Result};
use std::{convert::TryFrom, marker::PhantomData};
use thiserror::Error;

use crate::{language::Queryable, tree::Tree};

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
pub struct Query<T>
where
    T: Queryable,
{
    query: tree_sitter::Query,
    pub metavariables: Vec<Metavariable>,

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

impl<T> TryFrom<&str> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        RawQuery::new(value).to_query()
    }
}

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
    // public functions
    ////

    pub fn to_query(&self) -> Result<Query<T>> {
        let (qs, mvs) = self.to_query_string()?;
        let tsquery = tree_sitter::Query::new(T::target_language(), &qs)?;
        Ok(Query::new(tsquery, mvs))
    }

    pub fn to_query_string(&self) -> Result<(String, Vec<Metavariable>)> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(T::query_language())
            .expect("Error loading hcl-query grammar");

        let q = parser
            .parse(self.raw_bytes, None)
            .ok_or(anyhow!("failed to parse query string"))?;

        self.node_to_query_string(&q.root_node())
    }

    // internal functions
    ////

    fn node_to_query_string<'b>(
        &self,
        node: &'b tree_sitter::Node,
    ) -> Result<(String, Vec<Metavariable>)>
    where
        'a: 'b,
    {
        match node.kind() {
            "shisho_ellipsis" => Ok(("(_)+".into(), vec![])),
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
                Ok((format!("(_) @{}", variable_name), vec![metavariable]))
            }
            _ => {
                let children: Vec<tree_sitter::Node> = {
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor).collect()
                };

                if children.len() > 0 {
                    let mut child_queries = vec![];
                    let mut child_metavariables = vec![];

                    for child in children {
                        match self.node_to_query_string(&child) {
                            Ok((s, mv)) => {
                                child_queries.push(s);
                                child_metavariables.extend(mv);
                            }
                            Err(e) => return Err(e),
                        }
                    }

                    Ok((
                        format!(
                            r#"({} . {} .)"#,
                            node.kind(),
                            child_queries.into_iter().collect::<Vec<String>>().join("."),
                        ),
                        child_metavariables,
                    ))
                } else {
                    Ok((
                        format!(
                            r#"(({}) @{} (#eq? @{} "{}"))"#,
                            node.kind(),
                            node.id(),
                            node.id(),
                            self.node_as_value(node).replace("\"", "\\\"")
                        ),
                        vec![],
                    ))
                }
            }
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

pub struct QuerySession<'tree, 'query, T>
where
    T: Queryable,
    'tree: 'query,
{
    cursor: tree_sitter::QueryCursor,
    query: &'query Query<T>,
    tree: &'tree Tree<'tree, T>,
}

impl<'tree, 'query, T> QuerySession<'tree, 'query, T>
where
    T: Queryable,
    'tree: 'query,
{
    pub fn new(tree: &'tree Tree<'tree, T>, query: &'query Query<T>) -> Self {
        let cursor = tree_sitter::QueryCursor::new();
        QuerySession {
            tree,
            cursor,
            query,
        }
    }

    pub fn to_iter(&'query mut self) -> impl Iterator<Item = MatchedItem<'query>> + 'query {
        let raw = self.tree.raw;
        self.cursor.matches(
            self.query.ts_query(),
            self.tree.ts_tree().root_node(),
            move |x| x.utf8_text(raw).unwrap(),
        )
    }
}

pub type MatchedItem<'a> = tree_sitter::QueryMatch<'a>;
