use std::convert::TryFrom;
use thiserror::Error;

use crate::query::Query;

pub struct Tree<'a> {
    tree: tree_sitter::Tree,
    raw: &'a str,
}

pub type QueryCursor = tree_sitter::QueryCursor;
pub type QueryMatch<'a> = tree_sitter::QueryMatch<'a>;

impl<'a> Tree<'a> {
    pub fn new(tree: tree_sitter::Tree, raw: &'a str) -> Tree<'a> {
        Tree { tree, raw }
    }

    pub fn matches(
        &'a self,
        query: &'a Query,
        cursor: &'a mut QueryCursor,
    ) -> impl Iterator<Item = QueryMatch<'a>> {
        let raw_bytes = self.raw.as_bytes();
        cursor.matches(query.get_raw(), self.tree.root_node(), move |x| {
            x.utf8_text(raw_bytes).unwrap()
        })
    }
}

#[derive(Debug, Error)]
pub enum TreeError {
    #[error("ParseError: failed to parse query")]
    ParseError,

    #[error("ParseError: {0}")]
    ConvertError(tree_sitter::QueryError),
}

impl<'a> TryFrom<&'a str> for Tree<'a> {
    type Error = TreeError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(tree_sitter_hcl::language())
            .expect("Error loading hcl grammar");

        Ok(Tree::new(parser.parse(value, None).unwrap(), value))
    }
}
