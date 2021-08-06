use super::Queryable;

#[derive(Debug)]
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
        query::{Query, Pattern, TSQueryString},
        tree::RawTree,
    };

    use super::*;

    #[test]
    fn test_rawquery_conversion() {
        assert!(
            Pattern::<Go>::new(r#"for _, x := range iter { fmt.Printf("%s", x) }"#)
                .to_query_string()
                .is_ok()
        );
        assert!(Pattern::<Go>::new(
            r#"import "fmt"
            func main () { 
                x = []int{1, 2, 3}
                for _, y := range x {
                    fmt.Printf("%s", x) 
                } 
            }"#
        )
        .to_query_string()
        .is_ok());

        // with ellipsis operators
        assert!(Pattern::<Go>::new(
            r#"for _, x := range iter {
                :[...]
                fmt.Printf("%s", x)
                :[...]
            }"#
        )
        .to_query_string()
        .is_ok());

        // with metavariables
        {
            let rq = Pattern::<Go>::new(
                r#"for _, :[X] := range iter { 
                    :[...] 
                    fmt.Printf("%s", :[Y])
                    :[...]
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
        assert!(
            Pattern::<Go>::new(r#"for _, x := range iter { fmt.Printf("%s", x) }"#)
                .to_query()
                .is_ok()
        );
        assert!(Pattern::<Go>::new(
            r#"import "fmt"
            func main () { 
                x = []int{1, 2, 3}
                for _, y := range x {
                    fmt.Printf("%s", x) 
                } 
            }"#
        )
        .to_query()
        .is_ok());

        // with ellipsis operators
        assert!(Pattern::<Go>::new(
            r#"for _, x := range iter {
                :[...]
                fmt.Printf("%s", x)
                :[...]
            }"#
        )
        .to_query()
        .is_ok());

        // with metavariables
        {
            let rq = Pattern::<Go>::new(
                r#"for _, :[X] := range iter { 
                    :[...] 
                    fmt.Printf("%s", :[X])
                    :[...]
            }"#,
            )
            .to_query();
            assert!(rq.is_ok());
            let Query { metavariables, .. } = rq.unwrap();
            assert_eq!(metavariables.len(), 1);
        }
    }

    #[test]
    fn test_basic_query() {
        {
            let query = Pattern::<Go>::new(r#"for _, x := range iter { fmt.Printf("%s", x) }"#)
                .to_query()
                .unwrap();
            let tree = RawTree::<Go>::new(r#"for _, x := range iter { fmt.Printf("%s", x) }"#)
                .into_tree()
                .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }

        {
            let query =
                Pattern::<Go>::new(r#"for _, x := range iter { fmt.Printf("%s", :[VAR]) }"#)
                    .to_query()
                    .unwrap();
            let tree = RawTree::<Go>::new(r#"for _, x := range iter { fmt.Printf("%s", x) }"#)
                .into_tree()
                .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }
    }

    #[test]
    fn test_query_with_simple_metavariable() {
        {
            let query = Pattern::<Go>::new(
                r#"for _, :[VAR] := range iter {
                :[...]
            }"#,
            )
            .to_query()
            .unwrap();
            let tree = RawTree::<Go>::new(
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
            .into_tree()
            .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }

        {
            let query = Pattern::<Go>::new(
                r#"
                :[TMP] := :[X]
                :[X] = :[Y]
                :[Y] = :[TMP]
            "#,
            )
            .to_query()
            .unwrap();

            let tree = RawTree::<Go>::new(
                r#"
                x := def
                def = abc
                abc = x
            "#,
            )
            .into_tree()
            .unwrap();

            let session = tree.matches(&query);
            assert_eq!(session.collect().len(), 1);
        }
    }
}
