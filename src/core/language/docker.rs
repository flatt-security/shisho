use crate::core::node::{Node, NodeType, RootNode};

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

    fn unwrap_root<'tree, 'a>(root: &'a RootNode<'tree>) -> &'a Vec<Node<'tree>> {
        // see `//third_party/tree-sitter-dockerfile-query/grammar.js`
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
            NodeType::Normal("shell_fragment")
                | NodeType::Normal("double_quoted_string")
                | NodeType::Normal("unquoted_string")
                | NodeType::Normal("shell_command")
        )
    }

    fn node_value_eq<'a, 'b>(l: &Node<'a>, r: &Node<'b>) -> bool {
        if !l.is_named() && !r.is_named() {
            l.as_str().to_ascii_uppercase() == r.as_str().to_ascii_uppercase()
        } else {
            l.as_str() == r.as_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        matcher::MatchedItem,
        pattern::Pattern,
        query::{MetavariableId},
        tree::{Tree, TreeView},
    };
    use std::convert::TryFrom;

    #[test]
    fn test_from_instruction() {
        {
            let query = Pattern::<Dockerfile>::try_from(r#"FROM :[A]"#).unwrap();
            let query = query.as_query();
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
            let query = Pattern::<Dockerfile>::try_from(r#"FROM :[A]::[B]"#).unwrap();
            let query = query.as_query();
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
            let query = Pattern::<Dockerfile>::try_from(r#"FROM :[A]::[B]@:[HASH]"#).unwrap();
            let query = query.as_query();
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
                Pattern::<Dockerfile>::try_from(r#"FROM :[A]::[B]@:[HASH] as :[ALIAS]"#).unwrap();
            let query = query.as_query();
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
            let query = Pattern::<Dockerfile>::try_from(r#"RUN :[X]"#).unwrap();
            let query = query.as_query();
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
            let query = Pattern::<Dockerfile>::try_from(r#"RUN :[X]"#).unwrap();
            let query = query.as_query();
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
            let query = Pattern::<Dockerfile>::try_from(r#"COPY :[X] :[Y]"#).unwrap();
            let query = query.as_query();
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
