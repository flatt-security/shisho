mod docker;
mod go;
mod hcl;

use crate::core::node::Position;

pub use self::docker::Dockerfile;
pub use self::go::Go;
pub use self::hcl::HCL;

use super::node::{Node, Range};

pub trait Queryable {
    fn target_language() -> tree_sitter::Language;
    fn query_language() -> tree_sitter::Language;

    fn get_query_nodes<'tree, 'a>(root: &'a Box<Node<'tree>>) -> &'a Vec<Box<Node<'tree>>>;

    fn is_skippable(_node: &Box<Node>) -> bool {
        false
    }

    fn is_leaf_like(_node: &Box<Node>) -> bool {
        false
    }

    fn is_string_literal(_node: &Box<Node>) -> bool {
        false
    }

    fn range(node: &Box<Node>) -> Range {
        Self::default_range(node)
    }

    fn default_range(node: &Box<Node>) -> Range {
        if node.utf8_text().ends_with("\n") {
            Range {
                start: Position {
                    row: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                },
                end: Position {
                    row: node.end_position().row + 1,
                    column: 0,
                },
            }
        } else {
            Range {
                start: Position {
                    row: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                },
                end: Position {
                    row: node.end_position().row + 1,
                    column: node.end_position().column + 1,
                },
            }
        }
    }

    fn normalize_annonymous_leaf(s: &str) -> String {
        s.to_string()
    }
}
