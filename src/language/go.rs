use super::Queryable;

#[derive(Debug, Clone)]
pub struct Go;

impl Queryable for Go {
    fn target_language() -> tree_sitter::Language {
        tree_sitter_go::language()
    }

    fn query_language() -> tree_sitter::Language {
        tree_sitter_go_query::language()
    }

    fn get_query_nodes<'tree>(root: &'tree tree_sitter::Tree) -> Vec<tree_sitter::Node<'tree>> {
        // see `//third_party/tree-sitter-go-query/grammar.js`
        let source_file = root.root_node();

        let mut cursor = source_file.walk();
        source_file.children(&mut cursor).collect()
    }

    fn is_skippable(node: &tree_sitter::Node) -> bool {
        node.kind() == "\n"
    }

    fn is_leaf_like(node: &tree_sitter::Node) -> bool {
        Self::is_string_literal(node)
    }

    fn is_string_literal(node: &tree_sitter::Node) -> bool {
        match node.kind() {
            "interpreted_string_literal" | "raw_string_literal" => true,
            _ => false,
        }
    }

    fn range_for_view(node: &tree_sitter::Node) -> (tree_sitter::Point, tree_sitter::Point) {
        (node.start_position(), node.end_position())
    }
}

#[cfg(test)]
mod tests {
    use crate::matcher::MatchedItem;
    use crate::transform::Transformable;
    use crate::{
        code::Code,
        query::{MetavariableId, Query},
        tree::Tree,
    };
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn test_basic_query() {
        {
            let query =
                Query::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#).unwrap();
            let tree =
                Tree::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#).unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }

        {
            let query =
                Query::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", :[VAR]) }"#)
                    .unwrap();
            let tree =
                Tree::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#).unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }
    }

    #[test]
    fn test_query_with_simple_metavariable() {
        {
            let query = Query::<Go>::try_from(
                r#"for _, :[VAR] := range iter {
                :[...]
            }"#,
            )
            .unwrap();
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

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }

        {
            let query = Query::<Go>::try_from(
                r#"
                :[TMP] := :[X]
                :[X] = :[Y]
                :[Y] = :[TMP]
            "#,
            )
            .unwrap();

            let tree = Tree::<Go>::try_from(
                r#"
                x := def
                def = abc
                abc = x
            "#,
            )
            .unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect::<Vec<MatchedItem>>().len(), 1);
        }
    }

    #[test]
    fn test_function_call_expression() {
        {
            let query = Query::<Go>::try_from(r#"fmt.Printf("%s%d", :[X], 2)"#).unwrap();
            let tree = Tree::<Go>::try_from(r#"fmt.Printf("%s%d", "test", 2)"#).unwrap();
            let ptree = tree.to_partial();
            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("\"test\""));
        }
        {
            let query = Query::<Go>::try_from(r#"f("%s%d", :[...X])"#).unwrap();
            {
                let tree = Tree::<Go>::try_from(r#"f("%s%d", 1, 2)"#).unwrap();
                let ptree = tree.to_partial();
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("1, 2"));
            }
        }

        {
            let query = Query::<Go>::try_from(r#"f("%s%d", :[...X], 3)"#).unwrap();
            let tree = Tree::<Go>::try_from(r#"f("%s%d", 1, 2, 3)"#).unwrap();
            let ptree = tree.to_partial();
            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("1, 2"));
        }
    }

    #[test]
    fn test_object_call_expression() {
        {
            let tree = Tree::<Go>::try_from(r#"fmt.Printf("%s%d", "test", 2)"#).unwrap();
            let ptree = tree.to_partial();
            {
                let query = Query::<Go>::try_from(r#":[X].Printf("%s%d", :[...])"#).unwrap();
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("fmt"));
            }

            {
                let query = Query::<Go>::try_from(r#":[X]("%s%d", :[...])"#).unwrap();
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].value_of(&MetavariableId("X".into())),
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
            let ptree = tree.to_partial();
            {
                let query = Query::<Go>::try_from(
                    r#"func (:[X] *Receiver) f(a int, b string, c int) int { return 1 }"#,
                )
                .unwrap();
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("r"));
            }
            {
                let query =
                    Query::<Go>::try_from(r#"func (r *Receiver) f(:[...X]) int { return 1 }"#)
                        .unwrap();
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].value_of(&MetavariableId("X".into())),
                    Some("a int, b string, c int")
                );
            }
            {
                let query = Query::<Go>::try_from(
                    r#"func (r *Receiver) f(a int, :[...X], c int) int { return 1 }"#,
                )
                .unwrap();
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("b string"));
            }
        }
    }

    #[test]
    fn test_array() {
        {
            let tree = Tree::<Go>::try_from(r#"[]int {1, 2, 3, 4, 5}"#).unwrap();
            let ptree = tree.to_partial();
            {
                let query = Query::<Go>::try_from(r#"[] :[X] {1, 2, :[Y], 4, 5}"#).unwrap();
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("int"));
                assert_eq!(c[0].value_of(&MetavariableId("Y".into())), Some("3"));
            }
            {
                let query = Query::<Go>::try_from(r#"[] int {1, 2, :[...Y], 5}"#).unwrap();
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(c[0].value_of(&MetavariableId("Y".into())), Some("3, 4"));
            }
        }
    }

    #[test]
    fn test_string() {
        {
            let tree = Tree::<Go>::try_from(r#"a := "xoxp-test""#).unwrap();
            let ptree = tree.to_partial();
            let query = Query::<Go>::try_from(r#""xoxp-:[X]""#).unwrap();
            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("test"));
        }
        {
            let tree = Tree::<Go>::try_from(r#"a := `xoxp-test`"#).unwrap();
            let ptree = tree.to_partial();
            let query = Query::<Go>::try_from(r#"`xoxp-:[X]`"#).unwrap();
            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("test"));
        }
    }

    #[test]
    fn test_if() {
        {
            {
                let tree = Tree::<Go>::try_from(r#"if true == false { a := 2; b := 3 }"#).unwrap();
                let ptree = tree.to_partial();

                let query = Query::<Go>::try_from(r#"if :[X] { :[...Y] }"#).unwrap();
                let session = ptree.matches(&query);

                let c = session.collect::<Vec<MatchedItem>>();
                assert_eq!(c.len(), 1);
                assert_eq!(
                    c[0].value_of(&MetavariableId("X".into())),
                    Some("true == false")
                );
                assert_eq!(
                    c[0].value_of(&MetavariableId("Y".into())),
                    Some("a := 2; b := 3")
                );
            }
        }

        {
            let tree =
                Tree::<Go>::try_from(r#"if err := nil; true == false { a := 2; b := 3 }"#).unwrap();
            let ptree = tree.to_partial();

            let query = Query::<Go>::try_from(r#"if :[X]; :[Y] { :[...Z] }"#).unwrap();
            let session = ptree.matches(&query);

            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].value_of(&MetavariableId("X".into())),
                Some("err := nil")
            );
            assert_eq!(
                c[0].value_of(&MetavariableId("Y".into())),
                Some("true == false")
            );
            assert_eq!(
                c[0].value_of(&MetavariableId("Z".into())),
                Some("a := 2; b := 3")
            );
        }

        {
            let tree = Tree::<Go>::try_from(
                r#"if err := nil; true == false { a := 2; b := 3 } else { c := 4 }"#,
            )
            .unwrap();
            let ptree = tree.to_partial();
            let query = Query::<Go>::try_from(r#"if :[X] { :[...] }"#).unwrap();
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
        let ptree = tree.to_partial();

        let query = Query::<Go>::try_from(r#":[X] || :[X]"#).unwrap();

        let session = ptree.matches(&query);
        let mut c = session.collect::<Vec<MatchedItem>>();
        assert_eq!(c.len(), 1);

        let from_code = code.transform(c.pop().unwrap(), ":[X]");
        assert!(from_code.is_ok());

        assert_eq!(from_code.unwrap().as_str(), "func a() { b := 1 }",);
    }
}
