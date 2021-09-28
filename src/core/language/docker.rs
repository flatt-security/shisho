use crate::core::node::Node;

use super::Queryable;

#[derive(Debug, Clone)]
pub struct Dockerfile;

impl Queryable for Dockerfile {
    fn target_language() -> tree_sitter::Language {
        tree_sitter_dockerfile::language()
    }

    fn query_language() -> tree_sitter::Language {
        tree_sitter_dockerfile_query::language()
    }

    fn get_query_nodes<'tree, 'a>(root: &'a Box<Node<'tree>>) -> &'a Vec<Box<Node<'tree>>> {
        // see `//third_party/tree-sitter-dockerfile-query/grammar.js`
        &root.children
    }

    fn is_skippable(node: &Box<Node>) -> bool {
        node.kind() == "\n"
    }

    fn is_leaf_like(node: &Box<Node>) -> bool {
        Self::is_string_literal(node)
    }

    fn is_string_literal(node: &Box<Node>) -> bool {
        match node.kind() {
            "shell_fragment" | "double_quoted_string" | "unquoted_string" | "shell_command" => true,
            _ => false,
        }
    }

    fn normalize_annonymous_leaf(s: &str) -> String {
        s.to_ascii_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        matcher::MatchedItem,
        query::{MetavariableId, Query},
        tree::{Tree, TreeView},
    };
    use std::convert::TryFrom;

    #[test]
    fn test_from_instruction() {
        {
            let query = Query::<Dockerfile>::try_from(r#"FROM :[A]"#).unwrap();
            let tree = Tree::<Dockerfile>::try_from(r#"FROM name"#).unwrap();
            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("A".into()))
                    .map(|x| x.as_str()),
                Some("name")
            );
        }

        {
            let query = Query::<Dockerfile>::try_from(r#"FROM :[A]::[B]"#).unwrap();
            let tree = Tree::<Dockerfile>::try_from(r#"FROM name:tag"#).unwrap();
            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("A".into()))
                    .map(|x| x.as_str()),
                Some("name")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("B".into()))
                    .map(|x| x.as_str()),
                Some("tag")
            );
        }

        {
            let query = Query::<Dockerfile>::try_from(r#"FROM :[A]::[B]@:[HASH]"#).unwrap();
            let tree = Tree::<Dockerfile>::try_from(r#"FROM name:tag@hash"#).unwrap();
            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("A".into()))
                    .map(|x| x.as_str()),
                Some("name")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("B".into()))
                    .map(|x| x.as_str()),
                Some("tag")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("HASH".into()))
                    .map(|x| x.as_str()),
                Some("hash")
            );
        }

        {
            let query =
                Query::<Dockerfile>::try_from(r#"FROM :[A]::[B]@:[HASH] as :[ALIAS]"#).unwrap();
            let tree = Tree::<Dockerfile>::try_from(r#"FROM name:tag@hash as alias"#).unwrap();
            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("A".into()))
                    .map(|x| x.as_str()),
                Some("name")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("B".into()))
                    .map(|x| x.as_str()),
                Some("tag")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("HASH".into()))
                    .map(|x| x.as_str()),
                Some("hash")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("ALIAS".into()))
                    .map(|x| x.as_str()),
                Some("alias")
            );
        }
    }

    #[test]
    fn test_run_instruction() {
        {
            let query = Query::<Dockerfile>::try_from(r#"RUN :[X]"#).unwrap();
            let tree =
                Tree::<Dockerfile>::try_from(r#"RUN echo "hosts: files dns" > /etc/nsswitch.conf"#)
                    .unwrap();
            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some(r#"echo "hosts: files dns" > /etc/nsswitch.conf"#)
            );
        }
        {
            let query = Query::<Dockerfile>::try_from(r#"RUN :[X]"#).unwrap();
            let cmd = r#"RUN apt-get update && apt-get install -y \
            aufs-tools \
            automake \
            && rm -rf /var/lib/apt/lists/*"#;
            let tree = Tree::<Dockerfile>::try_from(cmd).unwrap();
            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some(cmd[4..].to_string().as_str())
            );
        }
    }

    #[test]
    fn test_copy_instruction() {
        {
            let query = Query::<Dockerfile>::try_from(r#"COPY :[X] :[Y]"#).unwrap();
            let tree = Tree::<Dockerfile>::try_from(r#"COPY ./ /app"#).unwrap();
            let ptree = TreeView::from(&tree);
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some("./")
            );
            assert_eq!(
                c[0].capture_of(&MetavariableId("Y".into()))
                    .map(|x| x.as_str()),
                Some("/app")
            );
        }
    }
}
