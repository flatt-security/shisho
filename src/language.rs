mod hcl;
pub use self::hcl::HCL;

pub trait Queryable {
    fn target_language() -> tree_sitter::Language;
    fn query_language() -> tree_sitter::Language;
}
