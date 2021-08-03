mod go;
mod hcl;
pub use self::go::Go;
pub use self::hcl::HCL;

pub trait Queryable {
    fn target_language() -> tree_sitter::Language;
    fn query_language() -> tree_sitter::Language;

    fn extract_query_nodes(root: &tree_sitter::Tree) -> Vec<tree_sitter::Node>;
}
