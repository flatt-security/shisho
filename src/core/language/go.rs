use super::Queryable;
use crate::core::node::{Node, NodeLike, NodeType, RootNode};

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

    fn is_skippable<N: NodeLike>(node: &N) -> bool {
        node.kind() == NodeType::Normal("\n")
    }

    fn is_leaf_like<N: NodeLike>(node: &N) -> bool {
        Self::is_string_literal(node)
    }

    fn is_string_literal<N: NodeLike>(node: &N) -> bool {
        matches!(
            node.kind(),
            NodeType::Normal("interpreted_string_literal") | NodeType::Normal("raw_string_literal")
        )
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::core::matcher::MatchedItem;
    use crate::core::pattern::Pattern;
    use crate::core::{query::MetavariableId, source::Code};
    use crate::match_pt;
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn test_basic_query() {
        match_pt!(
            Go,
            r#"for _, x := range iter { fmt.Printf("%s", x) }"#,
            r#"for _, x := range iter { fmt.Printf("%s", x) }"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );

        match_pt!(
            Go,
            r#"for _, x := range iter { fmt.Printf("%s", :[VAR]) }"#,
            r#"for _, x := range iter { fmt.Printf("%s", x) }"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );
    }

    #[test]
    fn test_query_with_simple_metavariable() {
        match_pt!(
            Go,
            r#"for _, :[VAR] := range iter {
                :[...]
            }"#,
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
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );

        match_pt!(
            Go,
            r#"
            :[TMP] := :[X]
            :[X] = :[Y]
            :[Y] = :[TMP]
            "#,
            r#"
                x := def
                def = abc
                abc = x
            "#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );
    }

    #[test]
    fn test_function_call_expression() {
        match_pt!(
            Go,
            r#"fmt.Printf("%s%d", :[X], 2)"#,
            r#"fmt.Printf("%s%d", "test", 2)"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("\"test\"".to_string())
                );
            }
        );

        match_pt!(
            Go,
            r#"f("%s%d", :[...X])"#,
            r#"f("%s%d", 1, 2)"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("1, 2".to_string())
                );
            }
        );

        match_pt!(
            Go,
            r#"f("%s%d", :[...X], 3)"#,
            r#"f("%s%d", 1, 2, 3)"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("1, 2".to_string())
                );
            }
        );
    }

    #[test]
    fn test_object_call_expression() {
        match_pt!(
            Go,
            r#":[X].Printf("%s%d", :[...])"#,
            r#"fmt.Printf("%s%d", "test", 2)"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("fmt".to_string())
                );
            }
        );

        match_pt!(
            Go,
            r#":[X]("%s%d", :[...])"#,
            r#"fmt.Printf("%s%d", "test", 2)"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("fmt.Printf".to_string())
                );
            }
        );
    }

    #[test]
    fn test_function_definitions() {
        match_pt!(
            Go,
            r#"func (:[X] *Receiver) f(a int, b string, c int) int { return 1 }"#,
            r#"func (r *Receiver) f(a int, b string, c int) int { return 1 }"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("r".to_string())
                );
            }
        );

        match_pt!(
            Go,
            r#"func (r *Receiver) f(:[...X]) int { return 1 }"#,
            r#"func (r *Receiver) f(a int, b string, c int) int { return 1 }"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("a int, b string, c int".to_string())
                );
            }
        );

        match_pt!(
            Go,
            r#"func (r *Receiver) f(a int, :[...X], c int) int { return 1 }"#,
            r#"func (r *Receiver) f(a int, b string, c int) int { return 1 }"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("b string".to_string())
                );
            }
        );
    }

    #[test]
    fn test_array() {
        match_pt!(
            Go,
            r#"[] :[X] {1, 2, :[Y], 4, 5}"#,
            r#"[]int {1, 2, 3, 4, 5}"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("int".to_string())
                );
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.to_string()),
                    Some("3".to_string())
                );
            }
        );

        match_pt!(
            Go,
            r#"[] int {1, 2, :[...Y], 5}"#,
            r#"[]int {1, 2, 3, 4, 5}"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.to_string()),
                    Some("3, 4".to_string())
                );
            }
        );
    }

    #[test]
    fn test_string() {
        match_pt!(Go, r#""xoxp-:[X]""#, r#"a := "xoxp-test""#, |c: Result<
            Vec<MatchedItem<Node<'_>>>,
        >| {
            let c = c.unwrap();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.to_string()),
                Some("test".to_string())
            );
        });

        match_pt!(Go, r#"`xoxp-:[X]`"#, r#"a := `xoxp-test`"#, |c: Result<
            Vec<MatchedItem<Node<'_>>>,
        >| {
            let c = c.unwrap();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.to_string()),
                Some("test".to_string())
            );
        });
    }

    #[test]
    fn test_if() {
        match_pt!(
            Go,
            r#"if :[X] { :[...Y] }"#,
            r#"if true == false { a := 2; b := 3 }"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );

        match_pt!(
            Go,
            r#"if :[X]; :[Y] { :[...Z] }"#,
            r#"if err := nil; true == false { a := 2; b := 3 }"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 1);
            }
        );

        match_pt!(
            Go,
            r#"if :[X] { :[...] }"#,
            r#"if err := nil; true == false { a := 2; b := 3 } else { c := 4 }"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let c = c.unwrap();
                assert_eq!(c.len(), 0);
            }
        );
    }

    #[test]
    fn basic_transform() {
        match_pt!(
            Go,
            r#":[X] || :[X]"#,
            r#"func a() { b := 1 || 1 }"#,
            |c: Result<Vec<MatchedItem<Node<'_>>>>| {
                let mut c = c.unwrap();

                assert_eq!(c.len(), 1);

                let code: Code<Go> = "func a() { b := 1 || 1 }".into();
                let autofix = Pattern::<Go>::try_from(":[X]").unwrap();
                let from_code = code.rewrite(autofix.as_rewrite_option(), &c.pop().unwrap());
                assert!(from_code.is_ok());

                assert_eq!(from_code.unwrap().as_str(), "func a() { b := 1 }",);
            }
        );
    }
}
