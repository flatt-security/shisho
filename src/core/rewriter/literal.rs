use crate::core::language::Queryable;
use crate::core::node::Node;
use crate::core::rewriter::builder::{Segment, SnippetBuilder};
use anyhow::Result;
use regex::Captures;

impl<'tree, T: Queryable> SnippetBuilder<'tree, T> {
    pub(crate) fn from_string_leaf(&self, node: &Node) -> Result<Segment> {
        let body = node.as_str().to_string();
        let r = regex::Regex::new(r":\[(\.\.\.)?(?P<name>[A-Z_][A-Z_0-9]*)\]").unwrap();
        let body = r.replace_all(body.as_str(), |caps: &Captures| {
            let name = caps.name("name").unwrap().as_str();
            self.from_metavariable(node, name)
                .map(|x| x.body)
                .unwrap_or_default()
        });
        Ok(Segment {
            body: body.into(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }
}
