mod docker;
mod go;
mod hcl;

use crate::core::node::{NodeLike, Position, Range};

pub use self::docker::Dockerfile;
pub use self::go::Go;
pub use self::hcl::HCL;

use super::pattern::{Pattern, PatternNode};

pub trait Queryable
where
    Self: Sized,
{
    fn target_language() -> tree_sitter::Language;
    fn query_language() -> tree_sitter::Language;

    /// `unwrap_root` takes a root of the query tree and returns nodes for matching.
    fn root_nodes<'tree>(pview: &'tree Pattern<'tree, Self>) -> Vec<&'tree PatternNode<'tree>>;

    /// `is_skippable` returns whether the given node could be ignored on matching.
    fn is_skippable<'tree, N: NodeLike<'tree>>(_node: &N) -> bool {
        false
    }

    fn is_leaf_like<'tree, N: NodeLike<'tree>>(_node: &N) -> bool {
        false
    }

    fn is_string_literal<'tree, N: NodeLike<'tree>>(_node: &N) -> bool {
        false
    }

    fn range<'tree, N: NodeLike<'tree>>(node: &N) -> Range {
        Self::default_range(node)
    }

    fn default_range<'tree, N: NodeLike<'tree>>(node: &N) -> Range {
        if node.as_cow().ends_with('\n') {
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

    fn node_value_eq<'nl, 'nr, NL: NodeLike<'nl>, NR: NodeLike<'nr>>(l: &NL, r: &NR) -> bool {
        *l.as_cow() == *r.as_cow()
    }
}

#[macro_export]
macro_rules! match_pt {
    ($lang:ident, $p:tt, $t:tt, $callback:expr) => {{
        let pc = crate::core::ruleset::constraint::PatternWithConstraints::new(crate::core::source::NormalizedSource::from($p), vec![]);

        let source = crate::core::source::NormalizedSource::from($t);
        let code = crate::core::tree::CST::<$lang>::try_from(&source).unwrap();
        let view = crate::core::tree::CSTView::from(&code);

        let query = crate::core::matcher::Query::try_from(&pc).unwrap();
        let session = view.find(&query);

        $callback(
            session.collect::<anyhow::Result<Vec<crate::core::matcher::MatchedItem<crate::core::node::CSTNode>>>>(),
        );
    }};
}

#[macro_export]
macro_rules! replace_pt {
    ($lang:ident, $p:tt, $t:tt, $r:tt, $callback:expr) => {{
        let pc = crate::core::ruleset::constraint::PatternWithConstraints::new(crate::core::source::NormalizedSource::from($p), vec![]);

        let source = crate::core::source::NormalizedSource::from($t);
        let code = crate::core::tree::CST::<$lang>::try_from(&source).unwrap();
        let view = crate::core::tree::CSTView::from(&code);

        let query = crate::core::matcher::Query::try_from(&pc).unwrap();
        let session = view.find(&query);
        let c = session.collect::<anyhow::Result<Vec<crate::core::matcher::MatchedItem<crate::core::node::CSTNode>>>>().unwrap();

        let pf = crate::core::ruleset::filter::PatternWithFilters::new(crate::core::source::NormalizedSource::from($r), vec![]);

        $callback(
            c.into_iter()
                .map(|amatch| Code::from($t).rewrite(&view, &amatch, crate::core::rewriter::RewriteOption::try_from(&pf)?))
                .collect::<anyhow::Result<Vec<crate::core::source::Code<$lang>>>>()
        );
    }};
}
