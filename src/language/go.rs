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

    fn extract_query_nodes<'tree>(root: &'tree tree_sitter::Tree) -> Vec<tree_sitter::Node<'tree>> {
        // see `//third_party/tree-sitter-go-query/grammar.js`
        let source_file = root.root_node();

        let mut cursor = source_file.walk();
        source_file.named_children(&mut cursor).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        query::{Query, TSQueryString},
        tree::Tree,
    };
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn test_rawquery_conversion() {
        assert!(
            TSQueryString::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#)
                .is_ok()
        );
        assert!(TSQueryString::<Go>::try_from(
            r#"import "fmt"
            func main () { 
                x = []int{1, 2, 3}
                for _, y := range x {
                    fmt.Printf("%s", x) 
                } 
            }"#
        )
        .is_ok());

        // with ellipsis operators
        assert!(TSQueryString::<Go>::try_from(
            r#"for _, x := range iter {
                :[...]
                fmt.Printf("%s", x)
                :[...]
            }"#
        )
        .is_ok());

        // with metavariables
        {
            let rq = TSQueryString::<Go>::try_from(
                r#"for _, :[X] := range iter { 
                    :[...] 
                    fmt.Printf("%s", :[Y])
                    :[...]
            }"#,
            );
            assert!(rq.is_ok());
            let TSQueryString { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 2);
        }
    }

    #[test]
    fn test_query_conversion() {
        assert!(Query::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#).is_ok());
        assert!(Query::<Go>::try_from(
            r#"import "fmt"
            func main () { 
                x = []int{1, 2, 3}
                for _, y := range x {
                    fmt.Printf("%s", x) 
                } 
            }"#
        )
        .is_ok());

        // with ellipsis operators
        assert!(Query::<Go>::try_from(
            r#"for _, x := range iter {
                :[...]
                fmt.Printf("%s", x)
                :[...]
            }"#
        )
        .is_ok());

        // with metavariables
        {
            let rq = Query::<Go>::try_from(
                r#"for _, :[X] := range iter { 
                    :[...] 
                    fmt.Printf("%s", :[X])
                    :[...]
            }"#,
            );
            assert!(rq.is_ok());
            let Query { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 1);
        }
    }

    #[test]
    fn test_basic_query() {
        {
            let query =
                Query::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#).unwrap();
            let tree =
                Tree::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#).unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }

        {
            let query =
                Query::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", :[VAR]) }"#)
                    .unwrap();
            let tree =
                Tree::<Go>::try_from(r#"for _, x := range iter { fmt.Printf("%s", x) }"#).unwrap();

            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            assert_eq!(session.collect().len(), 1);
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
            assert_eq!(session.collect().len(), 1);
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
            assert_eq!(session.collect().len(), 1);
        }
    }
}
