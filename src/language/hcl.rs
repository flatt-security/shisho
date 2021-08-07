use super::Queryable;

#[derive(Debug, Clone)]
pub struct HCL;

impl Queryable for HCL {
    fn target_language() -> tree_sitter::Language {
        tree_sitter_hcl::language()
    }

    fn query_language() -> tree_sitter::Language {
        tree_sitter_hcl_query::language()
    }

    fn extract_query_nodes<'tree>(root: &'tree tree_sitter::Tree) -> Vec<tree_sitter::Node<'tree>> {
        // TODO (y0n3uchy): this should be done more strictly.

        // see `//third_party/tree-sitter-hcl-query/grammar.js`
        let source_file = root.root_node();
        let body = source_file.child(0).unwrap();

        let mut cursor = source_file.walk();
        body.named_children(&mut cursor).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::transform::Transformable;
    use crate::{
        code::Code,
        pattern::Pattern,
        query::{Query, TSQueryString},
        tree::RawTree,
    };

    use super::*;

    #[test]
    fn test_rawquery_conversion() {
        assert!(Pattern::<HCL>::new(r#"test = "hoge""#)
            .to_query_string()
            .is_ok());
        assert!(
            Pattern::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query_string()
                .is_ok()
        );

        // with ellipsis operators
        assert!(Pattern::<HCL>::new(
            r#"resource "rtype" "rname" { :[...] attr = "value" :[...] }"#
        )
        .to_query_string()
        .is_ok());

        // with metavariables
        {
            let rq = Pattern::<HCL>::new(r#"resource "rtype" "rname" { attr = :[X] }"#)
                .to_query_string();
            assert!(rq.is_ok());
            let TSQueryString { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 1);

            let rq = Pattern::<HCL>::new(
                r#"resource "rtype" "rname" { 
                attr = :[X]
                :[...Y]
            }"#,
            )
            .to_query_string();
            assert!(rq.is_ok());
            let TSQueryString { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 2);
        }
    }

    #[test]
    fn test_query_conversion() {
        assert!(Pattern::<HCL>::new(r#"test = "hoge""#).to_query().is_ok());
        assert!(
            Pattern::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query()
                .is_ok()
        );

        // with ellipsis operators
        assert!(Pattern::<HCL>::new(
            r#"resource "rtype" "rname" { 
            :[...]
            attr = "value"
            :[...] 
        }"#
        )
        .to_query()
        .is_ok());

        // with metavariables
        {
            let rq = Pattern::<HCL>::new(r#"resource "rtype" "rname" { attr = :[X] }"#).to_query();
            assert!(rq.is_ok());
            assert_eq!(rq.unwrap().metavariables.len(), 1);

            let rq = Pattern::<HCL>::new(
                r#"resource "rtype" "rname" { 
                attr = :[X]
                :[...Y]
            }"#,
            )
            .to_query();
            assert!(rq.is_ok());
            let Query { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 2);
        }
    }

    #[test]
    fn test_basic_query() {
        {
            let query = Pattern::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .to_query()
                .unwrap();
            let tree = RawTree::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .into_tree()
                .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }

        {
            let query = Pattern::<HCL>::new(r#"resource "rtype" "rname" { attr = :[X] }"#)
                .to_query()
                .unwrap();
            let tree = RawTree::<HCL>::new(r#"resource "rtype" "rname" { attr = "value" }"#)
                .into_tree()
                .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }

        {
            let query = Pattern::<HCL>::new(
                r#"resource "rtype" "rname" { 
                attr = :[X]
                :[...Y]
            }"#,
            )
            .to_query()
            .unwrap();
            let tree = RawTree::<HCL>::new(
                r#"resource "rtype" "rname" { 
                attr = "value"
                hoge = "foobar"
                foo = "test"
            }"#,
            )
            .into_tree()
            .unwrap();

            let session = tree.matches(&query);
            let result = session.collect();
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].captures.len(), 2);
        }
    }

    #[test]
    fn test_query_with_simple_metavariable() {
        {
            let query = Pattern::<HCL>::new(r#"attr = :[X]"#).to_query().unwrap();
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

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 2);
        }

        {
            let query = Pattern::<HCL>::new(
                r#"
                one_attr = :[X]
                another_attr = :[Y]
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

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 2);
        }
    }

    #[test]
    fn test_query_with_ellipsis_opearator() {
        {
            let query = Pattern::<HCL>::new(
                r#"
                one_attr = :[X]
                :[...]
                another_attr = :[Y]
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

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 3);
        }
    }

    #[test]
    fn test_query_with_simple_equivalence() {
        {
            let query = Pattern::<HCL>::new(
                r#"
                one_attr = :[X]
                another_attr = :[X]
                yetanother_attr = :[X]
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

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }
    }

    #[test]
    fn basic_transform() {
        let code: Code<HCL> = "resource \"rtype\" \"rname\" { attr = \"notchanged\" }\nresource \"rtype\" \"another\" { attr = \"notchanged\" }".into();

        let tree_base = code.clone();
        let tree = RawTree::<HCL>::new(tree_base.as_str()).into_tree().unwrap();
        let query = Pattern::<HCL>::new(r#"resource "rtype" "rname" { attr = :[...] }"#)
            .to_query()
            .unwrap();

        let a = Pattern::<HCL>::new(r#"resource "rtype" "rname" { attr = :[...] }"#)
            .to_query_string()
            .unwrap();
        println!("query: {}", a.query_string);

        let item = {
            let session = tree.matches(&query);
            let mut items = session.collect();
            assert_eq!(items.len(), 1);
            items.pop().unwrap()
        };

        let new_code = code.transform(r#"resource "rtype" "rname" { attr = "changed" }"#, item);
        assert!(new_code.is_ok());

        assert_eq!(
            new_code.unwrap().as_str(),
            "resource \"rtype\" \"rname\" { attr = \"changed\" }\nresource \"rtype\" \"another\" { attr = \"notchanged\" }",
        );
    }

    #[test]
    fn metavariable_transform() {
        let code: Code<HCL> = "resource \"rtype\" \"rname\" { attr = \"one\" }\nresource \"rtype\" \"another\" { attr = \"two\" }".into();

        let tree_base = code.clone();
        let tree = RawTree::<HCL>::new(tree_base.as_str()).into_tree().unwrap();

        let query = Pattern::<HCL>::new("resource \"rtype\" \"rname\" { attr = :[X] }\nresource \"rtype\" \"another\" { attr = :[Y] }")
            .to_query()
            .unwrap();

        let item = {
            let session = tree.matches(&query);
            let mut items = session.collect();
            assert_eq!(items.len(), 1);
            items.pop().unwrap()
        };

        let new_code = code.transform("resource \"rtype\" \"rname\" { attr = :[Y] }\nresource \"rtype\" \"another\" { attr = :[X] }", item);
        assert!(new_code.is_ok());

        assert_eq!(
            new_code.unwrap().as_str(),
            "resource \"rtype\" \"rname\" { attr = \"two\" }\nresource \"rtype\" \"another\" { attr = \"one\" }",
        );
    }
}
