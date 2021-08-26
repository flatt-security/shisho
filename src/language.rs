mod go;
mod hcl;
pub use self::go::Go;
pub use self::hcl::HCL;

pub trait Queryable {
    fn target_language() -> tree_sitter::Language;
    fn query_language() -> tree_sitter::Language;

    fn get_query_nodes(root: &tree_sitter::Tree) -> Vec<tree_sitter::Node>;
    fn range_for_view(node: &tree_sitter::Node) -> (tree_sitter::Point, tree_sitter::Point);

    fn is_skippable(_node: &tree_sitter::Node) -> bool {
        false
    }

    fn is_leaf_like(_node: &tree_sitter::Node) -> bool {
        false
    }

    fn is_string_literal(_node: &tree_sitter::Node) -> bool {
        false
    }
}
