use std::convert::TryFrom;
use thiserror::Error;

pub struct Query {
    query: tree_sitter::Query,
}

impl Query {
    pub fn new(query: tree_sitter::Query) -> Query {
        Query { query }
    }

    pub fn get_raw<'a>(&'a self) -> &'a tree_sitter::Query {
        &self.query
    }
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("ParseError: failed to parse query")]
    ParseError,

    #[error("ParseError: {0}")]
    ConvertError(tree_sitter::QueryError),
}

impl TryFrom<&str> for Query {
    type Error = QueryError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(tree_sitter_hcl_query::language())
            .expect("Error loading hcl-query grammar");

        // TODO
        let tree = parser.parse(value, None).unwrap();

        // TODO: tree conversion

        tree_sitter::Query::new(
            tree_sitter_hcl_query::language(), // TODO: this should be tree_sitter_hcl::language(),
            tree.root_node().child(0).unwrap().to_sexp().as_str(),
        )
        .map(|q| Query::new(q))
        .map_err(|e| QueryError::ConvertError(e))
    }
}
