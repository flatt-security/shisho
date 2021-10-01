use super::Queryable;
use crate::core::node::{Node, NodeType, RootNode};

#[derive(Debug, Clone)]
pub struct Go;

impl Queryable for Go {
    fn target_language() -> tree_sitter::Language {
        tree_sitter_go::language()
    }

    fn query_language() -> tree_sitter::Language {
        tree_sitter_go_query::language()
    }

    fn unwrap_root<'tree, 'a>(root: &'a RootNode<'tree>) -> &'a Vec<Node<'tree>> {
        // see `//third_party/tree-sitter-go-query/grammar.js`
        &root.as_node().children
    }

    fn is_skippable(node: &Node) -> bool {
        node.kind() == NodeType::Normal("\n")
    }

    fn is_leaf_like(node: &Node) -> bool {
        Self::is_string_literal(node)
    }

    fn is_string_literal(node: &Node) -> bool {
        matches!(
            node.kind(),
            NodeType::Normal("interpreted_string_literal") | NodeType::Normal("raw_string_literal")
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::core::matcher::MatchedItem;
    use crate::core::pattern::Pattern;
    use crate::core::rewriter::Rewritable;
    use crate::core::tree::TreeView;
    use crate::core::{query::MetavariableId, source::Code, tree::Tree};
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn test_basic_query() {
        {
            let query =
                Pattern::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#)
                    .unwrap();
            let query = query.as_query();

            let tree =
                Tree::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#).unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }

        {
            let query =
                Pattern::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", :[VAR]) }"#)
                    .unwrap();
            let query = query.as_query();

            let tree =
                Tree::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#).unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }
    }

    #[test]
    fn test_query_with_simple_metavariable() {
        {
            let query = Pattern::<Go>::try_from(
                r#"for _, :[VAR] := range iter {
                :[...]
            }"#,
            )
            .unwrap();
            let query = query.as_query();

            let tree = Tree::<Go>::try_from(
                r#"
                for _, x := range iter { 
                    fmt.Printf("%s", x) 
                }
                for i, _ := range iter { 
                    fmt.Printf("%s", x) 
                }
                for i := range iter { 
                    fmt.Printf("%s", x) 
                }
                "#,
            )
            .unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }

        {
            let query = Pattern::<Go>::try_from(
                r#"
                :[TMP] := :[X]
                :[X] = :[Y]
                :[Y] = :[TMP]
            "#,
            )
            .unwrap();
            let query = query.as_query();

            let tree = Tree::<Go>::try_from(
                r#"
                x := def
                def = abc
                abc = x
            "#,
            )
            .unwrap();

            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }
    }

    #[test]
    fn test_function_call_expression() {
        {
            let query = Pattern::<Go>::try_from(r#"fmt.Printf("%s%d", :[X], 2)"#).unwrap();
            let query = query.as_query();

            let tree = Tree::<Go>::try_from(r#"fmt.Printf("%s%d", "test", 2)"#).unwrap();
            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("\"test\"")
            );
        }
        {
            let query = Pattern::<Go>::try_from(r#"f("%s%d", :[...X])"#).unwrap();
            let query = query.as_query();

            {
                let tree = Tree::<Go>::try_from(r#"f("%s%d", 1, 2)"#).unwrap();
                let ptree = TreeView::from(&tree);
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("1, 2")
                );
            }
        }

        {
            let query = Pattern::<Go>::try_from(r#"f("%s%d", :[...X], 3)"#).unwrap();
            let query = query.as_query();

            let tree = Tree::<Go>::try_from(r#"f("%s%d", 1, 2, 3)"#).unwrap();
            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("1, 2")
            );
        }
    }

    #[test]
    fn test_object_call_expression() {
        {
            let tree = Tree::<Go>::try_from(r#"fmt.Printf("%s%d", "test", 2)"#).unwrap();
            let ptree = TreeView::from(&tree);
            {
                let query = Pattern::<Go>::try_from(r#":[X].Printf("%s%d", :[...])"#).unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("fmt")
                );
            }

            {
                let query = Pattern::<Go>::try_from(r#":[X]("%s%d", :[...])"#).unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("fmt.Printf")
                );
            }
        }
        // TODO: support captures of chains
    }

    #[test]
    fn test_function_definitions() {
        {
            let tree = Tree::<Go>::try_from(
                r#"func (r *Receiver) f(a int, b string, c int) int { return 1 }"#,
            )
            .unwrap();
            let ptree = TreeView::from(&tree);
            {
                let query = Pattern::<Go>::try_from(
                    r#"func (:[X] *Receiver) f(a int, b string, c int) int { return 1 }"#,
                )
                .unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("r")
                );
            }
            {
                let query =
                    Pattern::<Go>::try_from(r#"func (r *Receiver) f(:[...X]) int { return 1 }"#)
                        .unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("a int, b string, c int")
                );
            }
            {
                let query = Pattern::<Go>::try_from(
                    r#"func (r *Receiver) f(a int, :[...X], c int) int { return 1 }"#,
                )
                .unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("b string")
                );
            }
        }
    }

    #[test]
    fn test_array() {
        {
            let tree = Tree::<Go>::try_from(r#"[]int {1, 2, 3, 4, 5}"#).unwrap();
            let ptree = TreeView::from(&tree);
            {
                let query = Pattern::<Go>::try_from(r#"[] :[X] {1, 2, :[Y], 4, 5}"#).unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("int")
                );
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.as_str()),
                    Some("3")
                );
            }
            {
                let query = Pattern::<Go>::try_from(r#"[] int {1, 2, :[...Y], 5}"#).unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.as_str()),
                    Some("3, 4")
                );
            }
        }
    }

    #[test]
    fn test_string() {
        {
            let tree = Tree::<Go>::try_from(r#"a := "xoxp-test""#).unwrap();
            let ptree = TreeView::from(&tree);

            let query = Pattern::<Go>::try_from(r#""xoxp-:[X]""#).unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("test")
            );
        }
        {
            let tree = Tree::<Go>::try_from(r#"a := `xoxp-test`"#).unwrap();
            let ptree = TreeView::from(&tree);

            let query = Pattern::<Go>::try_from(r#"`xoxp-:[X]`"#).unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("test")
            );
        }
    }

    #[test]
    fn test_if() {
        {
            {
                let tree = Tree::<Go>::try_from(r#"if true == false { a := 2; b := 3 }"#).unwrap();
                let ptree = TreeView::from(&tree);

                let query = Pattern::<Go>::try_from(r#"if :[X] { :[...Y] }"#).unwrap();
                let query = query.as_query();

                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("true == false")
                );
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.as_str()),
                    Some("a := 2; b := 3")
                );
            }
        }

        {
            let tree =
                Tree::<Go>::try_from(r#"if err := nil; true == false { a := 2; b := 3 }"#).unwrap();
            let ptree = TreeView::from(&tree);

            let query = Pattern::<Go>::try_from(r#"if :[X]; :[Y] { :[...Z] }"#).unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("err := nil")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("Y".into()))
                    .map(|x| x.as_str()),
                Some("true == false")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("Z".into()))
                    .map(|x| x.as_str()),
                Some("a := 2; b := 3")
            );
        }

        {
            let tree = Tree::<Go>::try_from(
                r#"if err := nil; true == false { a := 2; b := 3 } else { c := 4 }"#,
            )
            .unwrap();
            let ptree = TreeView::from(&tree);

            let query = Pattern::<Go>::try_from(r#"if :[X] { :[...] }"#).unwrap();
            let query = query.as_query();

            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 0);
        }
    }

    #[test]
    fn basic_transform() {
        let code: Code<Go> = "func a() { b := 1 || 1 }".into();

        let tree_base = code.clone();
        let tree = Tree::<Go>::try_from(tree_base.as_str()).unwrap();
        let ptree = TreeView::from(&tree);

        let query = Pattern::<Go>::try_from(r#":[X] || :[X]"#).unwrap();
        let query = query.as_query();

        let session = ptree.matches(&query);
        let mut c = session.collect::<Vec<MatchedItem>>();
        assert_eq!(c.len(), 1);

        let autofix = Pattern::<Go>::try_from(":[X]").unwrap();
        let from_code = code.to_rewritten_form(&c.pop().unwrap(), autofix.as_autofix());
        assert!(from_code.is_ok());

        assert_eq!(from_code.unwrap().as_str(), "func a() { b := 1 }",);
    }
}
