use crate::core::{
    node::{NodeLike, NodeType},
    pattern::{PatternNode, PatternView},
    tree::RootedTreeLike,
};

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

    fn root_nodes<'tree>(pview: &'tree PatternView<'tree, Self>) -> Vec<&'tree PatternNode<'tree>> {
        let root = pview.root().unwrap();
        let children = root.children(pview);
        let first = children
            .get(0)
            .expect("failed to load the code; no root element");
        first.children(pview)
    }

    fn is_leaf_like<'tree, N: NodeLike<'tree>>(node: &N) -> bool {
        Self::is_string_literal(node)
    }

    fn is_string_literal<'tree, N: NodeLike<'tree>>(node: &N) -> bool {
        matches!(
            node.kind(),
            NodeType::Normal("string_lit") | NodeType::Normal("quoted_template")
        )
    }

    fn is_skippable<'tree, N: NodeLike<'tree>>(node: &N) -> bool {
        node.kind() == NodeType::Normal("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::matcher::MatchedItem;
    use crate::core::node::CSTNode;
    use crate::core::query::MetavariableId;
    use crate::core::source::Code;
    use crate::{match_pt, replace_pt};
    use anyhow::Result;
    use std::convert::TryFrom;

    #[test]
    fn test_basic_query() {
        match_pt!(
            HCL,
            r#"encrypted = true"#,
            r#"encrypted = true"#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );

        match_pt!(
            HCL,
            r#"resource "rtype" "rname" { attr = "value" }"#,
            r#"resource "rtype" "rname" { attr = "value" }"#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );

        match_pt!(
            HCL,
            r#"resource "rtype" "rname" { attr = :[X] }"#,
            r#"resource "rtype" "rname" { attr = "value" }"#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );

        match_pt!(
            HCL,
            r#"resource "rtype" "rname" {
                attr = :[X]
                :[...Y]
            }"#,
            r#"resource "rtype" "rname" {
                attr = "value"
                hoge = "foobar"
                foo = "test"
            }"#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );
    }

    #[test]
    fn test_query_with_simple_metavariable() {
        match_pt!(
            HCL,
            r#"attr = :[X]"#,
            r#"resource "rtype" "rname" {
                attr = "value"
            }
            resource "rtype" "rname2" {
                another = "value"
            }
            resource "rtype" "rname3" {
                attr = "value"
            }"#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 2);
            }
        );

        match_pt!(
            HCL,
            r#"
                one_attr = :[X]
                another_attr = :[Y]
            "#,
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
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 2);
            }
        );
    }

    #[test]
    fn test_query_with_ellipsis_opearator() {
        match_pt!(
            HCL,
            r#"resource :[X] :[Y] {
                :[...]
               }"#,
            r#"
               resource "hoge" "foo" {
                   xx = 1
               }
           "#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );

        match_pt!(
            HCL,
            r#"
                one_attr = :[X]
                :[...]
                another_attr = :[Y]
            "#,
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
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 3);
            }
        );

        let cmd = r#"
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
        "#;
        match_pt!(
            HCL,
            r#"
                one_attr = :[X]
                another_attr = :[X]
                yetanother_attr = :[X]
            "#,
            cmd,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );

        match_pt!(
            HCL,
            r#"
                one_attr = :[_]
                another_attr = :[_]
                yetanother_attr = :[_]
            "#,
            cmd,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 2);
            }
        );
    }

    #[test]
    fn test_function_call() {
        match_pt!(
            HCL,
            r#"
                one_attr = max(1, :[X], 5)
            "#,
            r#"
                resource "rtype" "rname1" {
                    one_attr = max(1, 2, 5)
                }
            "#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("2".to_string())
                );
            }
        );

        match_pt!(
            HCL,
            r#"
                one_attr = max(1, :[...X], 5)
            "#,
            r#"
                resource "rtype" "rname1" {
                    one_attr = max(1, 2, 3, 4, 5)
                }
            "#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("2, 3, 4".to_string())
                );
            }
        );
    }

    #[test]
    fn test_attr() {
        match_pt!(
            HCL,
            r#"
                attr = :[X]
            "#,
            r#"
                resource "rtype" "rname1" {
                    attr = "hello1"
                }
                resource "rtype" "rname2" {
                    attr = "hello2"
                }
            "#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 2);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("\"hello1\"".to_string())
                );
                assert_eq!(
                    c[1].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("\"hello2\"".to_string())
                );
            }
        );
    }

    #[test]
    fn test_array() {
        match_pt!(
            HCL,
            r#"
                attr = [1, :[...X]]
            "#,
            r#"
                resource "rtype" "rname1" {
                    attr = [1, 2, 3, 4, 5]
                }
            "#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("2, 3, 4, 5".to_string())
                );
            }
        );

        match_pt!(
            HCL,
            r#"
                attr = [1, :[X], 3, :[Y], 5]
            "#,
            r#"
                resource "rtype" "rname1" {
                    attr = [1, 2, 3, 4, 5]
                }
            "#,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("2".to_string())
                );
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.to_string()),
                    Some("4".to_string())
                );
            }
        );
    }

    #[test]
    fn test_object() {
        match_pt!(
            HCL,
            r#"
                attr = {
                    :[...X]
                    key2 = :[Y]
                    :[...Z]
                }
            "#,
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
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("key1 = value1".to_string())
                );
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.to_string()),
                    Some("value2".to_string())
                );

                assert_eq!(
                    c[0].capture_of(&MetavariableId("Z".into()))
                        .map(|x| x.to_string()),
                    Some(
                        r#"key3 = value3
                        key4 = value4"#
                            .to_string()
                    )
                );
            }
        );
    }

    #[test]
    fn test_string() {
        let cmd = r#"
            resource "rtype" "rname1" {
                attr = "sample-0012-foo"
            }
        "#;

        match_pt!(
            HCL,
            r#"
                attr = "sample-:[X]-foo"
            "#,
            cmd,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("0012".to_string())
                );
            }
        );

        match_pt!(
            HCL,
            r#"
                attr = "sample-:[X]:[Y]-foo"
            "#,
            cmd,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("0012".to_string())
                );
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.to_string()),
                    Some("".to_string())
                );
            }
        );
    }

    #[test]
    fn test_for() {
        let cmd = r#"
            resource "rtype" "rname1" {
                attr = [for s in var.list : upper(s) if s != ""]
            }
            resource "rtype" "rname2" {
                attr = [for s, ss in var.list : upper(s) if s != ""]
            }
            resource "rtype" "rname3" {
                attr = {for s in var.list : s => upper(s) if s != ""}
            }
        "#;

        match_pt!(
            HCL,
            r#"
                attr = [for :[Y] in :[X] : upper(:[Y]) if :[Y] != ""]
            "#,
            cmd,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("var.list".to_string())
                );
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.to_string()),
                    Some("s".to_string())
                );
            }
        );

        match_pt!(
            HCL,
            r#"
                attr = [for :[...Y] in :[X] : upper(:[_]) if :[_] != ""]
            "#,
            cmd,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 2);
                assert_eq!(
                    c[1].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("var.list".to_string())
                );
                assert_eq!(
                    c[1].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.to_string()),
                    Some("s, ss".to_string())
                );
            }
        );

        match_pt!(
            HCL,
            r#"
                attr = {for :[...Y] in :[X] : :[_] => upper(:[_]) if :[_] != ""}
            "#,
            cmd,
            |c: Result<Vec<MatchedItem<CSTNode<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("var.list".to_string())
                );
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.to_string()),
                    Some("s".to_string())
                );
            }
        );
    }

    #[test]
    fn basic_transform() {
        replace_pt!(
            HCL,
            r#"
                resource "rtype" "rname" { attr = :[_] }
            "#,
            "resource \"rtype\" \"rname\" { attr = \"notchanged\" }\nresource \"rtype\" \"another\" { attr = \"notchanged\" }",
            "resource \"rtype\" \"rname\" { attr = \"changed\" }",
            |c: Result<Vec<Code<HCL>>>| {
                let mut c = c.unwrap();
                assert_eq!(c.pop().unwrap().as_str(),  "resource \"rtype\" \"rname\" { attr = \"changed\" }\nresource \"rtype\" \"another\" { attr = \"notchanged\" }");
            }
        );
    }

    #[test]
    fn metavariable_transform() {
        replace_pt!(
            HCL,
            "resource \"rtype\" \"rname\" { attr = :[X] }\nresource \"rtype\" \"another\" { attr = :[Y] }",
            "resource \"rtype\" \"rname\" { attr = \"one\" }\nresource \"rtype\" \"another\" { attr = \"two\" }",
            "resource \"rtype\" \"rname\" { attr = :[Y] }\nresource \"rtype\" \"another\" { attr = :[X] }",
            |c: Result<Vec<Code<HCL>>>| {
                let mut c = c.unwrap();
                assert_eq!(c.pop().unwrap().as_str(),  "resource \"rtype\" \"rname\" { attr = \"two\" }\nresource \"rtype\" \"another\" { attr = \"one\" }",);                         
            }
        );
    }
}
