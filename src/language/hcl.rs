use super::Queryable;

pub struct HCL;

impl Queryable for HCL {
    fn target_language() -> tree_sitter::Language {
        tree_sitter_hcl::language()
    }

    fn query_language() -> tree_sitter::Language {
        tree_sitter_hcl_query::language()
    }

    fn extract_query_nodes<'tree>(root: &'tree tree_sitter::Tree) -> Vec<tree_sitter::Node<'tree>> {
        // see `//third_party/tree-sitter-hcl-query/grammar.js`
        let source_file = root.root_node();
        let body = source_file.child(0).unwrap();

        let mut cursor = source_file.walk();
        body.named_children(&mut cursor).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        matcher::MatchedItem,
        query::{RawQuery, TSQueryString},
        tree::RawTree,
    };

    use super::*;

    #[test]
    fn test_rawquery_conversion() {
        assert!(RawQuery::<HCL>::new(r#"test = "hoge""#)
            .to_query_string()
            .is_ok());
        assert!(
            RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query_string()
                .is_ok()
        );

        // with ellipsis operators
        assert!(RawQuery::<HCL>::new(
            r#"resource "rtype" "rname" { :[[...]] attr = "value" :[[...]] }"#
        )
        .to_query_string()
        .is_ok());

        // with metavariables
        {
            let rq = RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = :[[X]] }"#)
                .to_query_string();
            assert!(rq.is_ok());
            let TSQueryString { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 1);
        }
    }

    #[test]
    fn test_query_conversion() {
        assert!(RawQuery::<HCL>::new(r#"test = "hoge""#).to_query().is_ok());
        assert!(
            RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query()
                .is_ok()
        );

        // with ellipsis operators
        assert!(RawQuery::<HCL>::new(
            r#"resource "rtype" "rname" { :[[...]] attr = "value" :[[...]] }"#
        )
        .to_query_string()
        .is_ok());

        // with metavariables
        {
            let rq =
                RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = :[[X]] }"#).to_query();
            assert!(rq.is_ok());
            assert_eq!(rq.unwrap().metavariables.len(), 1);
        }
    }

    #[test]
    fn test_basic_query() {
        {
            let query = RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query()
                .unwrap();
            let tree = RawTree::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .into_tree()
                .unwrap();

            let mut session = tree.matches(&query);
            assert_eq!(session.as_iter().collect::<Vec<MatchedItem>>().len(), 1);
        }

        {
            let query = RawQuery::<HCL>::new(r#"resource "rtype" "rname" { attr = :[[X]] }"#)
                .to_query()
                .unwrap();
            let tree = RawTree::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .into_tree()
                .unwrap();

            let mut session = tree.matches(&query);
            assert_eq!(session.as_iter().collect::<Vec<MatchedItem>>().len(), 1);
        }
    }

    #[test]
    fn test_query_with_simple_metavariable() {
        {
            let query = RawQuery::<HCL>::new(r#"attr = :[[X]]"#).to_query().unwrap();
            let tree = RawTree::<HCL>::new(
                r#"resource "rtype" "rname" { 
                attr = "value"
            }
            resource "rtype" "rname2" { 
                another = "value"
            }
            resource "rtype" "rname3" { 
                attr = "value"
            }"#,
            )
            .into_tree()
            .unwrap();

            let mut session = tree.matches(&query);
            assert_eq!(session.as_iter().collect::<Vec<MatchedItem>>().len(), 2);
        }

        {
            let query = RawQuery::<HCL>::new(
                r#"
                one_attr = :[[X]]
                another_attr = :[[Y]]
            "#,
            )
            .to_query()
            .unwrap();

            let tree = RawTree::<HCL>::new(
                r#"
                # should match
                resource "rtype" "rname1" { 
                    one_attr = "value"
                    another_attr = 2
                }

                # should NOT match
                resource "rtype" "rname2" { 
                    another_attr = 2
                }

                # should match
                resource "rtype" "rname3" { 
                    test = ""
                    one_attr = "value"
                    another_attr = 3
                    test = ""
                }

                # should NOT match
                resource "rtype" "rname4" { 
                    one_attr = "value"
                    test = ""
                    another_attr = 3
                }
            "#,
            )
            .into_tree()
            .unwrap();

            let mut session = tree.matches(&query);
            assert_eq!(session.as_iter().collect::<Vec<MatchedItem>>().len(), 2);
        }
    }

    #[test]
    fn test_query_with_ellipsis_opearator() {
        {
            let query = RawQuery::<HCL>::new(
                r#"
                one_attr = :[[X]]
                :[[...]]
                another_attr = :[[Y]]
            "#,
            )
            .to_query()
            .unwrap();

            let tree = RawTree::<HCL>::new(
                r#"
                # should match
                resource "rtype" "rname1" { 
                    one_attr = "value"
                    another_attr = 2
                }

                # should NOT match
                resource "rtype" "rname2" { 
                    another_attr = 2
                }

                # should match
                resource "rtype" "rname3" { 
                    test = ""
                    one_attr = "value"
                    another_attr = 3
                    test = ""
                }

                # should match
                resource "rtype" "rname4" { 
                    one_attr = "value"
                    test = ""
                    another_attr = 3
                }
            "#,
            )
            .into_tree()
            .unwrap();

            let mut session = tree.matches(&query);
            assert_eq!(session.as_iter().collect::<Vec<MatchedItem>>().len(), 3);
        }
    }

    #[test]
    fn test_query_with_simple_equivalence() {
        {
            let query = RawQuery::<HCL>::new(
                r#"
                one_attr = :[[X]]
                another_attr = :[[X]]
                yetanother_attr = :[[X]]
            "#,
            )
            .to_query()
            .unwrap();

            let tree = RawTree::<HCL>::new(
                r#"
                # should match
                resource "rtype" "rname1" { 
                    one_attr = "value"
                    another_attr = "value"
                    yetanother_attr = "value"
                }

                # should NOT match
                resource "rtype" "rname2" { 
                    another_attr = 2
                }

                # should NOT match
                resource "rtype" "rname3" { 
                    test = ""
                    one_attr = "value"
                    another_attr = 3
                    test = ""
                }
            "#,
            )
            .into_tree()
            .unwrap();

            let mut session = tree.matches(&query);
            assert_eq!(session.as_iter().collect::<Vec<MatchedItem>>().len(), 1);
        }
    }
}
