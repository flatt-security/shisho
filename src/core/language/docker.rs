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

    fn get_query_nodes<'tree>(root: &'tree tree_sitter::Tree) -> Vec<tree_sitter::Node<'tree>> {
        // TODO (y0n3uchy): this should be done more strictly.

        // see `//third_party/tree-sitter-dockerfile-query/grammar.js`
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
        tree::Tree,
    };
    use std::convert::TryFrom;

    #[test]
    fn test_from_instruction() {
        {
            let query = Query::<Dockerfile>::try_from(r#"FROM :[A]"#).unwrap();
            let tree = Tree::<Dockerfile>::try_from(r#"FROM name"#).unwrap();
            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("A".into())), Some("name"));
        }

        {
            let query = Query::<Dockerfile>::try_from(r#"FROM :[A]::[B]"#).unwrap();
            let tree = Tree::<Dockerfile>::try_from(r#"FROM name:tag"#).unwrap();
            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("A".into())), Some("name"));
            assert_eq!(c[0].value_of(&MetavariableId("B".into())), Some("tag"));
        }

        {
            let query = Query::<Dockerfile>::try_from(r#"FROM :[A]::[B]@:[HASH]"#).unwrap();

            let tree = Tree::<Dockerfile>::try_from(r#"FROM name:tag@hash"#).unwrap();
            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("A".into())), Some("name"));
            assert_eq!(c[0].value_of(&MetavariableId("B".into())), Some("tag"));
            assert_eq!(c[0].value_of(&MetavariableId("HASH".into())), Some("hash"));
        }

        {
            let query =
                Query::<Dockerfile>::try_from(r#"FROM :[A]::[B]@:[HASH] as :[ALIAS]"#).unwrap();

            let tree = Tree::<Dockerfile>::try_from(r#"FROM name:tag@hash as alias"#).unwrap();
            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("A".into())), Some("name"));
            assert_eq!(c[0].value_of(&MetavariableId("B".into())), Some("tag"));
            assert_eq!(c[0].value_of(&MetavariableId("HASH".into())), Some("hash"));
            assert_eq!(
                c[0].value_of(&MetavariableId("ALIAS".into())),
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
            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].value_of(&MetavariableId("X".into())),
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
            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(
                c[0].value_of(&MetavariableId("X".into())),
                Some(cmd[4..].to_string().as_str())
            );
        }
    }

    #[test]
    fn test_copy_instruction() {
        {
            let query = Query::<Dockerfile>::try_from(r#"COPY :[X] :[Y]"#).unwrap();

            let tree = Tree::<Dockerfile>::try_from(r#"COPY ./ /app"#).unwrap();
            let ptree = tree.to_partial();
            let session = ptree.matches(&query);
            let c = session.collect::<Vec<MatchedItem>>();
            assert_eq!(c.len(), 1);
            assert_eq!(c[0].value_of(&MetavariableId("X".into())), Some("./"));
            assert_eq!(c[0].value_of(&MetavariableId("Y".into())), Some("/app"));
        }
    }
}
