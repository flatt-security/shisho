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
    use crate::tree::Tree;
    use crate::{
        code::Code,
        query::{Query, TSQueryString},
    };
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn test_rawquery_conversion() {
        assert!(TSQueryString::<HCL>::try_from(r#"test = "hoge""#).is_ok());
        assert!(
            TSQueryString::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#)
                .is_ok()
        );

        // with ellipsis operators
        assert!(TSQueryString::<HCL>::try_from(
            r#"resource "rtype" "rname" { :[...] attr = "value" :[...] }"#
        )
        .is_ok());

        // with metavariables
        {
            let rq = TSQueryString::<HCL>::try_from(r#"resource "rtype" "rname" { attr = :[X] }"#);
            assert!(rq.is_ok());
            let TSQueryString { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 1);

            let rq = TSQueryString::<HCL>::try_from(
                r#"resource "rtype" "rname" { 
                attr = :[X]
                :[...Y]
            }"#,
            );
            assert!(rq.is_ok());
            let TSQueryString { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 2);
        }
    }

    #[test]
    fn test_query_conversion() {
        assert!(Query::<HCL>::try_from(r#"test = "hoge""#).is_ok());
        assert!(Query::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).is_ok());

        // with ellipsis operators
        assert!(Query::<HCL>::try_from(
            r#"resource "rtype" "rname" { 
            :[...]
            attr = "value"
            :[...] 
        }"#
        )
        .is_ok());

        // with metavariables
        {
            let rq = TSQueryString::<HCL>::try_from(r#"resource "rtype" "rname" { attr = :[X] }"#);
            assert!(rq.is_ok());
            assert_eq!(rq.unwrap().metavariables.len(), 1);

            let rq = Query::<HCL>::try_from(
                r#"resource "rtype" "rname" { 
                attr = :[X]
                :[...Y]
            }"#,
            );
            assert!(rq.is_ok());
            let Query { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 2);
        }
    }

    #[test]
    fn test_basic_query() {
        {
            let query =
                Query::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).unwrap();
            let tree =
                Tree::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }

        {
            let query =
                Query::<HCL>::try_from(r#"resource "rtype" "rname" { attr = :[X] }"#).unwrap();
            let tree =
                Tree::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }

        {
            let query = Query::<HCL>::try_from(
                r#"resource "rtype" "rname" { 
                attr = :[X]
                :[...Y]
            }"#,
            )
            .unwrap();
            let tree = Tree::<HCL>::try_from(
                r#"resource "rtype" "rname" { 
                attr = "value"
                hoge = "foobar"
                foo = "test"
            }"#,
            )
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
            let query = Query::<HCL>::try_from(r#"attr = :[X]"#).unwrap();
            let tree = Tree::<HCL>::try_from(
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
            .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 2);
        }

        {
            let query = Query::<HCL>::try_from(
                r#"
                one_attr = :[X]
                another_attr = :[Y]
            "#,
            )
            .unwrap();

            let tree = Tree::<HCL>::try_from(
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
            .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 2);
        }
    }

    #[test]
    fn test_query_with_ellipsis_opearator() {
        {
            let query = Query::<HCL>::try_from(
                r#"
                one_attr = :[X]
                :[...]
                another_attr = :[Y]
            "#,
            )
            .unwrap();

            let tree = Tree::<HCL>::try_from(
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
            .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 3);
        }
    }

    #[test]
    fn test_query_with_simple_equivalence() {
        {
            let query = Query::<HCL>::try_from(
                r#"
                one_attr = :[X]
                another_attr = :[X]
                yetanother_attr = :[X]
            "#,
            )
            .unwrap();

            let tree = Tree::<HCL>::try_from(
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
            .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }
    }

    #[test]
    fn basic_transform() {
        let code: Code<HCL> = "resource \"rtype\" \"rname\" { attr = \"notchanged\" }\nresource \"rtype\" \"another\" { attr = \"notchanged\" }".into();

        let tree_base = code.clone();
        let tree = Tree::<HCL>::try_from(tree_base.as_str()).unwrap();
        let query =
            Query::<HCL>::try_from(r#"resource "rtype" "rname" { attr = :[...] }"#).unwrap();

        let item = {
            let session = tree.matches(&query);
            let mut items = session.collect();
            assert_eq!(items.len(), 1);
            items.pop().unwrap()
        };

        let from_code = code.transform(r#"resource "rtype" "rname" { attr = "changed" }"#, item);
        assert!(from_code.is_ok());

        assert_eq!(
            from_code.unwrap().as_str(),
            "resource \"rtype\" \"rname\" { attr = \"changed\" }\nresource \"rtype\" \"another\" { attr = \"notchanged\" }",
        );
    }

    #[test]
    fn metavariable_transform() {
        let code = Code::<HCL>::from("resource \"rtype\" \"rname\" { attr = \"one\" }\nresource \"rtype\" \"another\" { attr = \"two\" }");

        let tree_base = code.clone();
        let tree = Tree::<HCL>::try_from(tree_base.as_str()).unwrap();

        let query = Query::<HCL>::try_from("resource \"rtype\" \"rname\" { attr = :[X] }\nresource \"rtype\" \"another\" { attr = :[Y] }")        
            .unwrap();

        let item = {
            let session = tree.matches(&query);
            let mut items = session.collect();
            assert_eq!(items.len(), 1);
            items.pop().unwrap()
        };

        let from_code = code.transform("resource \"rtype\" \"rname\" { attr = :[Y] }\nresource \"rtype\" \"another\" { attr = :[X] }", item);
        assert!(from_code.is_ok());

        assert_eq!(
            from_code.unwrap().as_str(),
            "resource \"rtype\" \"rname\" { attr = \"two\" }\nresource \"rtype\" \"another\" { attr = \"one\" }",
        );
    }
}
