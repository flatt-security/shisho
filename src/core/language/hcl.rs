use crate::core::node::Range;

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

    fn get_query_nodes<'tree>(root: &'tree tree_sitter::Tree) -> Vec<tree_sitter::Node<'tree>> {
        // TODO (y0n3uchy): this should be done more strictly.

        // see `//third_party/tree-sitter-hcl-query/grammar.js`
        let source_file = root.root_node();
        let body = source_file
            .child(0)
            .expect("failed to load the code; no root element");

        let mut cursor = source_file.walk();
        body.children(&mut cursor).collect()
    }

    fn is_leaf_like(node: &tree_sitter::Node) -> bool {
        Self::is_string_literal(node)
    }

    fn is_string_literal(node: &tree_sitter::Node) -> bool {
        match node.kind() {
            "string_literal" | "quoted_template" => true,
            _ => false,
        }
    }

    fn is_skippable(node: &tree_sitter::Node) -> bool {
        node.kind() == "\n"
    }

    fn range(node: &tree_sitter::Node, source: &[u8]) -> Range {
        match node.kind() {
            "attribute" => {
                let bracket = node
                    .children(&mut node.walk())
                    .skip(node.child_count() - 2)
                    .next()
                    .unwrap();
                let start = Self::default_range(node, source).start;
                let end = Self::range(&bracket, source).end;
                Range { start, end }
            }
            "block" => {
                let bracket = node
                    .children(&mut node.walk())
                    .skip(node.child_count() - 2)
                    .next()
                    .unwrap();
                let start = Self::default_range(node, source).start;
                let end = Self::range(&bracket, source).end;
                Range { start, end }
            }
            _ => Self::default_range(node, source),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::matcher::MatchedItem;
    use crate::core::query::MetavariableId;
    use crate::core::transform::Transformable;
    use crate::core::tree::Tree;
    use crate::core::{code::Code, query::Query};
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn test_basic_query() {
        {
            let query = Query::<HCL>::try_from(r#"encrypted = true"#).unwrap();
            let tree = Tree::<HCL>::try_from(r#"encrypted = true"#).unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }
        {
            let query =
                Query::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).unwrap();
            let tree =
                Tree::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }

        {
            let query =
                Query::<HCL>::try_from(r#"resource "rtype" "rname" { attr = :[X] }"#).unwrap();
            let tree =
                Tree::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
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

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let result = session.collect::<Vec<MatchedItem>>();
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

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 2);
        }

        {
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
            let ptree = tree.to_partial();

            let query = Query::<HCL>::try_from(
                r#"
                one_attr = :[X]
                another_attr = :[Y]
            "#,
            )
            .unwrap();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 2);
        }
    }

    #[test]
    fn test_query_with_ellipsis_opearator() {
        {
            let query = Query::<HCL>::try_from(
                r#"resource :[X] :[Y] {
                :[...]
               }"#,
            )
            .unwrap();
            let tree = Tree::<HCL>::try_from(
                r#"
                resource "hoge" "foo" {
                    xx = 1
                }
            "#,
            )
            .unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }
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

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 3);
        }
        {
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

                # should NOT match
                resource "rtype" "rname1" { 
                    one_attr = "value"
                    another_attr = "value"
                    yetanother_attr = "changed"
                }
            "#,
            )
            .unwrap();
            let ptree = tree.to_partial();

            {
                let query = Query::<HCL>::try_from(
                    r#"
                    one_attr = :[X]
                    another_attr = :[X]
                    yetanother_attr = :[X]
                "#,
                )
                .unwrap();
                let session = ptree.matches(&query);
                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
            }
            {
                let query = Query::<HCL>::try_from(
                    r#"
                    one_attr = :[_]
                    another_attr = :[_]
                    yetanother_attr = :[_]
                "#,
                )
                .unwrap();
                let session = ptree.matches(&query);
                assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 2);
            }
        }
    }

    #[test]
    fn test_function_call() {
        {
            let query = Query::<HCL>::try_from(
                r#"
                one_attr = max(1, :[X], 5)
            "#,
            )
            .unwrap();

            let tree = Tree::<HCL>::try_from(
                r#"
                resource "rtype" "rname1" { 
                    one_attr = max(1, 2, 5)
                }
            "#,
            )
            .unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("2"));
        }

        {
            let query = Query::<HCL>::try_from(
                r#"
                one_attr = max(1, :[...X], 5)
            "#,
            )
            .unwrap();

            let tree = Tree::<HCL>::try_from(
                r#"
                resource "rtype" "rname1" { 
                    one_attr = max(1, 2, 3, 4, 5)
                }
            "#,
            )
            .unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("2, 3, 4"));
        }
    }

    #[test]
    fn test_attr() {
        {
            let query = Query::<HCL>::try_from(
                r#"
                attr = :[X]
            "#,
            )
            .unwrap();

            let tree = Tree::<HCL>::try_from(
                r#"
                resource "rtype" "rname1" { 
                    attr = "hello1"
                }
                resource "rtype" "rname2" { 
                    attr = "hello2" 
                }
            "#,
            )
            .unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 2);
            assert_eq!(
                c[0].value_of(&MetavariableId("X".into())),
                Some("\"hello1\"")
            );
            assert_eq!(
                c[1].value_of(&MetavariableId("X".into())),
                Some("\"hello2\"")
            );
        }
    }

    #[test]
    fn test_array() {
        let tree = Tree::<HCL>::try_from(
            r#"
            resource "rtype" "rname1" { 
                attr = [1, 2, 3, 4, 5]
            }                
        "#,
        )
        .unwrap();
        let ptree = tree.to_partial();
        {
            let query = Query::<HCL>::try_from(
                r#"
                attr = [1, :[...X]]
            "#,
            )
            .unwrap();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].value_of(&MetavariableId("X".into())),
                Some("2, 3, 4, 5")
            );
        }
        {
            let query = Query::<HCL>::try_from(
                r#"
                attr = [1, :[X], 3, :[Y], 5]
            "#,
            )
            .unwrap();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("2"));
            assert_eq!(c[0].value_of(&MetavariableId("Y".into())), Some("4"));
        }
    }

    #[test]
    fn test_object() {
        let tree = Tree::<HCL>::try_from(
            r#"
            resource "rtype" "rname1" { 
                attr = {
                    key1 = value1
                    key2 = value2
                    key3 = value3
                    key4 = value4
                }
            }                
        "#,
        )
        .unwrap();
        let ptree = tree.to_partial();
        {
            let query = Query::<HCL>::try_from(
                r#"
                attr = { 
                    :[...X]
                    key2 = :[Y]
                    :[...Z]
                }
            "#,
            )
            .unwrap();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].value_of(&MetavariableId("X".into())),
                Some("key1 = value1")
            );
            assert_eq!(c[0].value_of(&MetavariableId("Y".into())), Some("value2"));

            assert_eq!(
                c[0].value_of(&MetavariableId("Z".into())),
                Some(
                    r#"key3 = value3
                    key4 = value4"#
                )
            );
        }
    }

    #[test]
    fn test_string() {
        {
            let tree = Tree::<HCL>::try_from(
                r#"
                resource "rtype" "rname1" { 
                    attr = "sample-0012-foo"
                }                
            "#,
            )
            .unwrap();
            let ptree = tree.to_partial();
            {
                let query = Query::<HCL>::try_from(
                    r#"
                attr = "sample-:[X]-foo"
            "#,
                )
                .unwrap();
                let session = ptree.matches(&query);
                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("0012"));
            }

            {
                let query = Query::<HCL>::try_from(
                    r#"
                attr = "sample-:[X]:[Y]-foo"
            "#,
                )
                .unwrap();
                let session = ptree.matches(&query);
                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("0012"));
                assert_eq!(c[0].value_of(&MetavariableId("Y".into())), Some(""));
            }
        }
    }

    #[test]
    fn test_for() {
        let tree = Tree::<HCL>::try_from(
            r#"
            resource "rtype" "rname1" {                
                attr = [for s in var.list : upper(s) if s != ""]
            }
            resource "rtype" "rname2" {                
                attr = [for s, ss in var.list : upper(s) if s != ""]
            }            
            resource "rtype" "rname3" { 
                attr = {for s in var.list : s => upper(s) if s != ""}
            }                
        "#,
        )
        .unwrap();
        let ptree = tree.to_partial();
        {
            let query = Query::<HCL>::try_from(
                r#"
                attr = [for :[Y] in :[X] : upper(:[Y]) if :[Y] != ""]
                }
            "#,
            )
            .unwrap();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("var.list"));
            assert_eq!(c[0].value_of(&MetavariableId("Y".into())), Some("s"));
        }
        {
            let query = Query::<HCL>::try_from(
                r#"
                attr = [for :[...Y] in :[X] : upper(:[_]) if :[_] != ""]
                }
            "#,
            )
            .unwrap();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 2);
            assert_eq!(c[1].value_of(&MetavariableId("X".into())), Some("var.list"));
            assert_eq!(c[1].value_of(&MetavariableId("Y".into())), Some("s, ss"));
        }
        {
            let query = Query::<HCL>::try_from(
                r#"
                attr = {for :[...Y] in :[X] : :[_] => upper(:[_]) if :[_] != ""}
                }
            "#,
            )
            .unwrap();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("var.list"));
            assert_eq!(c[0].value_of(&MetavariableId("Y".into())), Some("s"));
        }
    }

    #[test]
    fn basic_transform() {
        let code: Code<HCL> = "resource \"rtype\" \"rname\" { attr = \"notchanged\" }\nresource \"rtype\" \"another\" { attr = \"notchanged\" }".into();

        let tree_base = code.clone();
        let tree = Tree::<HCL>::try_from(tree_base.as_str()).unwrap();
        let ptree = tree.to_partial();

        let query = Query::<HCL>::try_from(r#"resource "rtype" "rname" { attr = :[_] }"#).unwrap();
        let session = ptree.matches(&query);
        let mut c = session.collect::<Vec<MatchedItem>>();
        assert_eq!(c.len(), 1);

        let from_code = code.transform(
            &c.pop().unwrap(),
            "resource \"rtype\" \"rname\" { attr = \"changed\" }\n",
        );
        assert!(from_code.is_ok());

        assert_eq!(
            from_code.unwrap().as_str(),
            "resource \"rtype\" \"rname\" { attr = \"changed\" }\nresource \"rtype\" \"another\" { attr = \"notchanged\" }",
        );
    }

    #[test]
    fn metavariable_transform() {
        let code = Code::<HCL>::from("resource \"rtype\" \"rname\" { attr = \"one\" }\nresource \"rtype\" \"another\" { attr = \"two\" }\n");

        let tree_base = code.clone();
        let tree = Tree::<HCL>::try_from(tree_base.as_str()).unwrap();
        let ptree = tree.to_partial();

        let query = Query::<HCL>::try_from("resource \"rtype\" \"rname\" { attr = :[X] }\nresource \"rtype\" \"another\" { attr = :[Y] }\n")
            .unwrap();

        let session = ptree.matches(&query);
        let mut c = session.collect::<Vec<MatchedItem>>();
        assert_eq!(c.len(), 1);

        let from_code = code.transform(&c.pop().unwrap(), "resource \"rtype\" \"rname\" { attr = :[Y] }\nresource \"rtype\" \"another\" { attr = :[X] }\n");
        assert!(from_code.is_ok());

        assert_eq!(
            from_code.unwrap().as_str(),
            "resource \"rtype\" \"rname\" { attr = \"two\" }\nresource \"rtype\" \"another\" { attr = \"one\" }\n",
        );
    }
}
