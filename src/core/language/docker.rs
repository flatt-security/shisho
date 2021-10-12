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
    use crate::{
        core::{matcher::MatchedItem, query::MetavariableId},
        match_pt,
    };
    use anyhow::Result;
    use std::convert::TryFrom;

    #[test]
    fn test_from_instruction() {
        match_pt!(
            Dockerfile,
            r#"FROM :[A]"#,
            r#"FROM name"#,
            |matches: Result<Vec<MatchedItem>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("A".into()))
                        .map(|x| x.as_str()),
                    Some("name")
                );
            }
        );

        match_pt!(
            Dockerfile,
            r#"FROM :[A]::[B]"#,
            r#"FROM name:tag"#,
            |matches: Result<Vec<MatchedItem>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("A".into()))
                        .map(|x| x.as_str()),
                    Some("name")
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("B".into()))
                        .map(|x| x.as_str()),
                    Some("tag")
                );
            }
        );

        match_pt!(
            Dockerfile,
            r#"FROM :[A]::[B]@:[HASH]"#,
            r#"FROM name:tag@hash"#,
            |matches: Result<Vec<MatchedItem>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("A".into()))
                        .map(|x| x.as_str()),
                    Some("name")
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("B".into()))
                        .map(|x| x.as_str()),
                    Some("tag")
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("HASH".into()))
                        .map(|x| x.as_str()),
                    Some("hash")
                );
            }
        );

        match_pt!(
            Dockerfile,
            r#"FROM :[A]::[B]@:[HASH] as :[ALIAS]"#,
            r#"FROM name:tag@hash as alias"#,
            |matches: Result<Vec<MatchedItem>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("A".into()))
                        .map(|x| x.as_str()),
                    Some("name")
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("B".into()))
                        .map(|x| x.as_str()),
                    Some("tag")
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("HASH".into()))
                        .map(|x| x.as_str()),
                    Some("hash")
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("ALIAS".into()))
                        .map(|x| x.as_str()),
                    Some("alias")
                );
            }
        );
    }

    #[test]
    fn test_run_instruction() {
        match_pt!(
            Dockerfile,
            r#"RUN :[X]"#,
            r#"RUN echo "hosts: files dns" > /etc/nsswitch.conf"#,
            |matches: Result<Vec<MatchedItem>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some(r#"echo "hosts: files dns" > /etc/nsswitch.conf"#)
                );
            }
        );

        let cmd = r#"RUN apt-get update && apt-get install -y \
        aufs-tools \
        automake \
        && rm -rf /var/lib/apt/lists/*"#;
        match_pt!(Dockerfile, r#"RUN :[X]"#, cmd, |matches: Result<
            Vec<MatchedItem>,
        >| {
            let matches = matches.unwrap();
            assert_eq!(matches.len(), 1);
            assert_eq!(
                matches[0]
                    .capture_of(&MetavariableId("X".into()))
                    .map(|x| x.as_str()),
                Some(cmd[4..].to_string().as_str())
            );
        });
    }

    #[test]
    fn test_copy_instruction() {
        match_pt!(
            Dockerfile,
            r#"COPY :[X] :[Y]"#,
            r#"COPY ./ /app"#,
            |matches: Result<Vec<MatchedItem>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("X".into()))
                        .map(|x| x.as_str()),
                    Some("./")
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.as_str()),
                    Some("/app")
                );
            }
        );
    }
}
