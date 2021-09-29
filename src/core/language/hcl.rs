use crate::core::node::{Node, NodeType, Range, RootNode};

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

    fn unwrap_root<'tree, 'a>(root: &'a RootNode<'tree>) -> &'a Vec<Node<'tree>> {
        // see `//third_party/tree-sitter-hcl-query/grammar.js`
        &root
            .as_node()
            .children
            .get(0)
            .expect("failed to load the code; no root element")
            .children
    }

    fn is_leaf_like(node: &Node) -> bool {
        Self::is_string_literal(node)
    }

    fn is_string_literal(node: &Node) -> bool {
        matches!(
            node.kind(),
            NodeType::Normal("string_literal") | NodeType::Normal("quoted_template")
        )
    }

    fn is_skippable(node: &Node) -> bool {
        node.kind() == NodeType::Normal("\n")
    }

    fn range(node: &Node) -> Range {
        match node.kind() {
            NodeType::Normal("attribute") => {
                let bracket = node.children.get(node.children.len() - 2).unwrap();
                let start = Self::default_range(node).start;
                let end = Self::range(bracket).end;
                Range { start, end }
            }
            NodeType::Normal("block") => {
                let bracket = node.children.get(node.children.len() - 2).unwrap();
                let start = Self::default_range(node).start;
                let end = Self::range(bracket).end;
                Range { start, end }
            }
            _ => Self::default_range(node),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::matcher::MatchedItem;
    use crate::core::pattern::Pattern;
    use crate::core::query::MetavariableId;
    use crate::core::source::Code;
    use crate::core::transform::Transformable;
    use crate::core::tree::{Tree, TreeView};
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn test_basic_query() {
        {
            let query = Pattern::<HCL>::try_from(r#"encrypted = true"#).unwrap();
            let query = query.as_query();

            let tree = Tree::<HCL>::try_from(r#"encrypted = true"#).unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }
        {
            let query =
                Pattern::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).unwrap();
            let query = query.as_query();

            let tree =
                Tree::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }

        {
            let query =
                Pattern::<HCL>::try_from(r#"resource "rtype" "rname" { attr = :[X] }"#).unwrap();
            let query = query.as_query();

            let tree =
                Tree::<HCL>::try_from(r#"resource "rtype" "rname" { attr = "value" }"#).unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }

        {
            let query = Pattern::<HCL>::try_from(
                r#"resource "rtype" "rname" { 
                attr = :[X]
                :[...Y]
            }"#,
            )
            .unwrap();
            let query = query.as_query();

            let tree = Tree::<HCL>::try_from(
                r#"resource "rtype" "rname" { 
                attr = "value"
                hoge = "foobar"
                foo = "test"
            }"#,
            )
            .unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let result = session.collect::<Vec<MatchedItem>>();
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].captures.len(), 2);
        }
    }

    #[test]
    fn test_query_with_simple_metavariable() {
        {
            let query = Pattern::<HCL>::try_from(r#"attr = :[X]"#).unwrap();
            let query = query.as_query();

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

            let ptree = TreeView::from(&tree);
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
            let ptree = TreeView::from(&tree);

            let query = Pattern::<HCL>::try_from(
                r#"
                one_attr = :[X]
                another_attr = :[Y]
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 2);
        }
    }

    #[test]
    fn test_query_with_ellipsis_opearator() {
        {
            let query = Pattern::<HCL>::try_from(
                r#"resource :[X] :[Y] {
                :[...]
               }"#,
            )
            .unwrap();
            let query = query.as_query();
            let tree = Tree::<HCL>::try_from(
                r#"
                resource "hoge" "foo" {
                    xx = 1
                }
            "#,
            )
            .unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }
        {
            let query = Pattern::<HCL>::try_from(
                r#"
                one_attr = :[X]
                :[...]
                another_attr = :[Y]
            "#,
            )
            .unwrap();
            let query = query.as_query();

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

            let ptree = TreeView::from(&tree);
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
            let ptree = TreeView::from(&tree);

            {
                let query = Pattern::<HCL>::try_from(
                    r#"
                    one_attr = :[X]
                    another_attr = :[X]
                    yetanother_attr = :[X]
                "#,
                )
                .unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);
                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
            }
            {
                let query = Pattern::<HCL>::try_from(
                    r#"
                    one_attr = :[_]
                    another_attr = :[_]
                    yetanother_attr = :[_]
                "#,
                )
                .unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);
                assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 2);
            }
        }
    }

    #[test]
    fn test_function_call() {
        {
            let query = Pattern::<HCL>::try_from(
                r#"
                one_attr = max(1, :[X], 5)
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let tree = Tree::<HCL>::try_from(
                r#"
                resource "rtype" "rname1" { 
                    one_attr = max(1, 2, 5)
                }
            "#,
            )
            .unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("2")
            );
        }

        {
            let query = Pattern::<HCL>::try_from(
                r#"
                one_attr = max(1, :[...X], 5)
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let tree = Tree::<HCL>::try_from(
                r#"
                resource "rtype" "rname1" { 
                    one_attr = max(1, 2, 3, 4, 5)
                }
            "#,
            )
            .unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("2, 3, 4")
            );
        }
    }

    #[test]
    fn test_attr() {
        {
            let query = Pattern::<HCL>::try_from(
                r#"
                attr = :[X]
            "#,
            )
            .unwrap();
            let query = query.as_query();

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

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 2);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("\"hello1\"")
            );
            assert_eq!(
                c[1].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
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
        let ptree = TreeView::from(&tree);
        {
            let query = Pattern::<HCL>::try_from(
                r#"
                attr = [1, :[...X]]
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("2, 3, 4, 5")
            );
        }
        {
            let query = Pattern::<HCL>::try_from(
                r#"
                attr = [1, :[X], 3, :[Y], 5]
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("2")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("Y".into()))
                    .map(|x| x.as_str()),
                Some("4")
            );
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
        let ptree = TreeView::from(&tree);
        {
            let query = Pattern::<HCL>::try_from(
                r#"
                attr = { 
                    :[...X]
                    key2 = :[Y]
                    :[...Z]
                }
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("key1 = value1")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("Y".into()))
                    .map(|x| x.as_str()),
                Some("value2")
            );

            assert_eq!(
                c[0].capture_of(&MetavariableId("Z".into()))
                    .map(|x| x.as_str()),
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
            let ptree = TreeView::from(&tree);
            {
                let query = Pattern::<HCL>::try_from(
                    r#"
                attr = "sample-:[X]-foo"
            "#,
                )
                .unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);
                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("0012")
                );
            }

            {
                let query = Pattern::<HCL>::try_from(
                    r#"
                attr = "sample-:[X]:[Y]-foo"
            "#,
                )
                .unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);
                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("0012")
                );
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.as_str()),
                    Some("")
                );
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
        let ptree = TreeView::from(&tree);
        {
            let query = Pattern::<HCL>::try_from(
                r#"
                attr = [for :[Y] in :[X] : upper(:[Y]) if :[Y] != ""]
                }
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("var.list")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("Y".into()))
                    .map(|x| x.as_str()),
                Some("s")
            );
        }
        {
            let query = Pattern::<HCL>::try_from(
                r#"
                attr = [for :[...Y] in :[X] : upper(:[_]) if :[_] != ""]
                }
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 2);
            assert_eq!(
                c[1].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("var.list")
            );
            assert_eq!(
                c[1].capture_of(&MetavariableId("Y".into()))
                    .map(|x| x.as_str()),
                Some("s, ss")
            );
        }
        {
            let query = Pattern::<HCL>::try_from(
                r#"
                attr = {for :[...Y] in :[X] : :[_] => upper(:[_]) if :[_] != ""}
                }
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("var.list")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("Y".into()))
                    .map(|x| x.as_str()),
                Some("s")
            );
        }
    }

    #[test]
    fn basic_transform() {
        let code: Code<HCL> = "resource \"rtype\" \"rname\" { attr = \"notchanged\" }\nresource \"rtype\" \"another\" { attr = \"notchanged\" }".into();

        let tree_base = code.clone();
        let tree = Tree::<HCL>::try_from(tree_base.as_str()).unwrap();
        let ptree = TreeView::from(&tree);

        let query =
            Pattern::<HCL>::try_from(r#"resource "rtype" "rname" { attr = :[_] }"#).unwrap();
        let query = query.as_query();

        let session = ptree.matches(&query);
        let mut c = session.collect::<Vec<MatchedItem>>();
        assert_eq!(c.len(), 1);

        let autofix =
            Pattern::<HCL>::try_from("resource \"rtype\" \"rname\" { attr = \"changed\" }\n")
                .unwrap();
        let from_code = code.transform(&c.pop().unwrap(), autofix.as_autofix());
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
        let ptree = TreeView::from(&tree);

        let query = Pattern::<HCL>::try_from("resource \"rtype\" \"rname\" { attr = :[X] }\nresource \"rtype\" \"another\" { attr = :[Y] }\n")
            .unwrap();
        let query = query.as_query();

        let session = ptree.matches(&query);
        let mut c = session.collect::<Vec<MatchedItem>>();
        assert_eq!(c.len(), 1);

        let autofix = Pattern::<HCL>::try_from("resource \"rtype\" \"rname\" { attr = :[Y] }\nresource \"rtype\" \"another\" { attr = :[X] }\n").unwrap();
        let from_code = code.transform(&c.pop().unwrap(), autofix.as_autofix());
        assert!(from_code.is_ok());

        assert_eq!(
            from_code.unwrap().as_str(),
            "resource \"rtype\" \"rname\" { attr = \"two\" }\nresource \"rtype\" \"another\" { attr = \"one\" }\n",
        );
    }
}
