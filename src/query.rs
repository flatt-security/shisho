use anyhow::{anyhow, Result};
use std::convert::TryFrom;
use thiserror::Error;

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
pub struct Query {
    query: tree_sitter::Query,
    pub metavariables: Vec<Metavariable>,
}

impl Query {
    pub fn new(query: tree_sitter::Query, metavariables: Vec<Metavariable>) -> Self {
        Query {
            query,
            metavariables,
        }
    }

    pub fn ts_query<'a>(&'a self) -> &'a tree_sitter::Query {
        &self.query
    }
}

impl TryFrom<&str> for Query {
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
struct RawQuery<'a> {
    raw_bytes: &'a [u8],
}

impl<'a> RawQuery<'a> {
    pub fn new(raw_str: &'a str) -> Self {
        RawQuery {
            raw_bytes: raw_str.as_bytes(),
        }
    }

    // public functions
    ////

    pub fn to_query(&self) -> Result<Query> {
        let (qs, mvs) = self.to_query_string()?;
        let tsquery = tree_sitter::Query::new(tree_sitter_hcl::language(), &qs)?;
        Ok(Query::new(tsquery, mvs))
    }

    pub fn to_query_string(&self) -> Result<(String, Vec<Metavariable>)> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(tree_sitter_hcl_query::language())
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

impl<'a> From<&'a str> for RawQuery<'a> {
    fn from(value: &'a str) -> Self {
        RawQuery::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rawquery_conversion() {
        assert!(RawQuery::new(r#"test = "hoge""#).to_query_string().is_ok());
        assert!(
            RawQuery::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query_string()
                .is_ok()
        );

        // with ellipsis operators
        assert!(
            RawQuery::new(r#"resource "rtype" "rname" { ... attr = "value" ... }"#)
                .to_query_string()
                .is_ok()
        );

        // with metavariables
        {
            let rq = RawQuery::new(r#"resource "rtype" "rname" { attr = $X }"#).to_query_string();
            assert!(rq.is_ok());
            let (_, metavariables) = rq.unwrap();
            assert_eq!(metavariables.len(), 1);
        }
    }

    #[test]
    fn test_query_conversion() {
        assert!(RawQuery::new(r#"test = "hoge""#).to_query().is_ok());
        assert!(
            RawQuery::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query()
                .is_ok()
        );

        // with ellipsis operators
        assert!(
            RawQuery::new(r#"resource "rtype" "rname" { ... attr = "value" ... }"#)
                .to_query_string()
                .is_ok()
        );

        // with metavariables
        {
            let rq = RawQuery::new(r#"resource "rtype" "rname" { attr = $X }"#).to_query();
            assert!(rq.is_ok());
            assert_eq!(rq.unwrap().metavariables.len(), 1);
        }
    }
}
