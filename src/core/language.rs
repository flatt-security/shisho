mod docker;
mod go;
mod hcl;

use crate::core::node::{Node, NodeLike, Position, Range, RootNode};

pub use self::docker::Dockerfile;
pub use self::go::Go;
pub use self::hcl::HCL;

pub trait Queryable {
    fn target_language() -> tree_sitter::Language;
    fn query_language() -> tree_sitter::Language;

    /// `unwrap_root` takes a root of the query tree and returns nodes for matching.
    fn unwrap_root<'tree, 'a>(root: &'a RootNode<'tree>) -> &'a Vec<Node<'tree>>;

    /// `is_skippable` returns whether the given node could be ignored on matching.
    fn is_skippable<'a, N: NodeLike<'a>>(_node: &N) -> bool {
        false
    }

    fn is_leaf_like<'a, N: NodeLike<'a>>(_node: &N) -> bool {
        false
    }

    fn is_string_literal<'a, N: NodeLike<'a>>(_node: &N) -> bool {
        false
    }

    fn range<'a, N: NodeLike<'a>>(node: &N) -> Range {
        Self::default_range(node)
    }

    fn default_range<'a, N: NodeLike<'a>>(node: &N) -> Range {
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

    fn node_value_eq<'a, 'b, NL: NodeLike<'a>, NR: NodeLike<'b>>(l: &NL, r: &NR) -> bool {
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
        let ptree = crate::core::tree::NormalizedTree::from(&tree);
        let ptree = ptree.as_ref_treeview();
        let session = ptree.matches(&query);

        $callback(session.collect::<anyhow::Result<Vec<crate::core::matcher::MatchedItem>>>());
    }};
}
