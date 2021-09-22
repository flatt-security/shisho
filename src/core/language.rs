mod docker;
mod go;
mod hcl;

use crate::core::node::Position;

pub use self::docker::Dockerfile;
pub use self::go::Go;
pub use self::hcl::HCL;

use super::node::Range;

pub trait Queryable {
    fn target_language() -> tree_sitter::Language;
    fn query_language() -> tree_sitter::Language;

    fn get_query_nodes(root: &tree_sitter::Tree) -> Vec<tree_sitter::Node>;

    fn is_skippable(_node: &tree_sitter::Node) -> bool {
        false
    }

    fn is_leaf_like(_node: &tree_sitter::Node) -> bool {
        false
    }

    fn is_string_literal(_node: &tree_sitter::Node) -> bool {
        false
    }

    fn range(node: &tree_sitter::Node, source: &[u8]) -> Range {
        Self::default_range(node, source)
    }

    fn default_range(node: &tree_sitter::Node, source: &[u8]) -> Range {
        if node
            .utf8_text(&[source, "\n".as_bytes()].concat())
            .unwrap()
            .ends_with("\n")
        {
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
