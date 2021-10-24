use super::Queryable;
use crate::core::node::{Node, NodeLike, NodeType, RootNode};

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

    fn is_skippable<N: NodeLike>(node: &N) -> bool {
        node.kind() == NodeType::Normal("\n")
    }

    fn is_leaf_like<N: NodeLike>(node: &N) -> bool {
        Self::is_string_literal(node)
    }

    fn is_string_literal<N: NodeLike>(node: &N) -> bool {
        matches!(
            node.kind(),
            NodeType::Normal("shell_fragment")
                | NodeType::Normal("double_quoted_string")
                | NodeType::Normal("unquoted_string")
                | NodeType::Normal("shell_command")
        )
    }

    fn node_value_eq<NL: NodeLike, NR: NodeLike>(l: &NL, r: &NR) -> bool {
        l.as_cow().to_ascii_uppercase() == r.as_cow().to_ascii_uppercase()
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
            |matches: Result<Vec<MatchedItem<Node<'_>>>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("A".into()))
                        .map(|x| x.to_string()),
                    Some("name".to_string())
                );
            }
        );

        match_pt!(
            Dockerfile,
            r#"FROM :[A]::[B]"#,
            r#"FROM name:tag"#,
            |matches: Result<Vec<MatchedItem<Node<'_>>>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("A".into()))
                        .map(|x| x.to_string()),
                    Some("name".to_string())
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("B".into()))
                        .map(|x| x.to_string()),
                    Some("tag".to_string())
                );
            }
        );

        match_pt!(
            Dockerfile,
            r#"FROM :[A]::[B]@:[HASH]"#,
            r#"FROM name:tag@hash"#,
            |matches: Result<Vec<MatchedItem<Node<'_>>>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("A".into()))
                        .map(|x| x.to_string()),
                    Some("name".to_string())
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("B".into()))
                        .map(|x| x.to_string()),
                    Some("tag".to_string())
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("HASH".into()))
                        .map(|x| x.to_string()),
                    Some("hash".to_string())
                );
            }
        );

        match_pt!(
            Dockerfile,
            r#"FROM :[A]::[B]@:[HASH] as :[ALIAS]"#,
            r#"FROM name:tag@hash as alias"#,
            |matches: Result<Vec<MatchedItem<Node<'_>>>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("A".into()))
                        .map(|x| x.to_string()),
                    Some("name".to_string())
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("B".into()))
                        .map(|x| x.to_string()),
                    Some("tag".to_string())
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("HASH".into()))
                        .map(|x| x.to_string()),
                    Some("hash".to_string())
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("ALIAS".into()))
                        .map(|x| x.to_string()),
                    Some("alias".to_string())
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
            |matches: Result<Vec<MatchedItem<Node<'_>>>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some(r#"echo "hosts: files dns" > /etc/nsswitch.conf"#.to_string())
                );
            }
        );

        let cmd = r#"RUN apt-get update && apt-get install -y \
        aufs-tools \
        automake \
        && rm -rf /var/lib/apt/lists/*"#;
        match_pt!(Dockerfile, r#"RUN :[X]"#, cmd, |matches: Result<
            Vec<MatchedItem<Node<'_>>>,
        >| {
            let matches = matches.unwrap();
            assert_eq!(matches.len(), 1);
            assert_eq!(
                matches[0]
                    .capture_of(&MetavariableId("X".into()))
                    .map(|x| x.to_string()),
                Some(cmd[4..].to_string())
            );
        });
    }

    #[test]
    fn test_copy_instruction() {
        match_pt!(
            Dockerfile,
            r#"COPY :[X] :[Y]"#,
            r#"COPY ./ /app"#,
            |matches: Result<Vec<MatchedItem<Node<'_>>>>| {
                let matches = matches.unwrap();
                assert_eq!(matches.len(), 1);
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("X".into()))
                        .map(|x| x.to_string()),
                    Some("./".to_string())
                );
                assert_eq!(
                    matches[0]
                        .capture_of(&MetavariableId("Y".into()))
                        .map(|x| x.to_string()),
                    Some("/app".to_string())
                );
            }
        );
    }
}
