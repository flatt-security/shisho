mod docker;
mod go;
mod hcl;

use crate::core::node::Position;

pub use self::docker::Dockerfile;
pub use self::go::Go;
pub use self::hcl::HCL;

use super::node::{Node, Range, RootNode};

pub trait Queryable {
    fn target_language() -> tree_sitter::Language;
    fn query_language() -> tree_sitter::Language;

    /// `unwrap_root` takes a root of the query tree and returns nodes for matching.
    fn unwrap_root<'tree, 'a>(root: &'a RootNode<'tree>) -> &'a Vec<Node<'tree>>;

    /// `is_skippable` returns whether the given node could be ignored on matching.
    fn is_skippable(_node: &Node) -> bool {
        false
    }

    fn is_leaf_like(_node: &Node) -> bool {
        false
    }

    fn is_string_literal(_node: &Node) -> bool {
        false
    }

    fn range(node: &Node) -> Range {
        Self::default_range(node)
    }

    fn default_range(node: &Node) -> Range {
        if node.as_str().ends_with('\n') {
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

    fn node_value_eq<'a, 'b>(l: &Node<'a>, r: &Node<'b>) -> bool {
        *l.as_str() == *r.as_str()
    }
}

#[macro_export]
macro_rules! match_pt {
    ($lang:ident, $p:tt, $t:tt, $callback:expr) => {{
        let pattern = crate::core::pattern::Pattern::<$lang>::try_from($p).unwrap();
        let pc = crate::core::pattern::PatternWithConstraints::new(pattern, vec![]);

        let query = pc.as_query();
        let tree = crate::core::tree::Tree::<$lang>::try_from($t).unwrap();
        let ptree = crate::core::tree::TreeView::from(&tree);
        let session = ptree.matches(&query);

        $callback(session.collect::<anyhow::Result<Vec<crate::core::matcher::MatchedItem>>>());
    }};
}
