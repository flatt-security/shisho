use crate::core::{
    language::Queryable,
    matcher::CaptureItem,
    matcher::MatchedItem,
    node::NodeType,
    node::{Node, RootNode},
    query::MetavariableId,
};
use anyhow::{anyhow, Result};
use thiserror::Error;

use super::RewriteOption;

pub struct SnippetBuilder<'pattern, T>
where
    T: Queryable,
{
    autofix: &'pattern RewriteOption<'pattern, T>,
    item: &'pattern MatchedItem<'pattern>,
}

impl<'pattern, T> SnippetBuilder<'pattern, T>
where
    T: Queryable,
{
    pub fn new(
        autofix: &'pattern RewriteOption<'pattern, T>,
        item: &'pattern MatchedItem<'pattern>,
    ) -> Self {
        Self { autofix, item }
    }
}

#[derive(Debug)]
struct Segment {
    pub body: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug)]
pub struct Snippet {
    pub body: String,
}

#[derive(Debug, Error)]
enum SnippetBuilderError {
    #[error("MetavariableError: {id}")]
    MetavariableUnavailable {
        id: String,
        start_byte: usize,
        end_byte: usize,
    },
}

impl<'tree, T> SnippetBuilder<'tree, T>
where
    T: Queryable,
{
    pub fn from_root(&self, rnode: &RootNode) -> Result<Snippet, anyhow::Error> {
        let pitems = T::unwrap_root(rnode)
            .iter()
            .map(|node| (node, self.from_node(node)))
            .collect::<Vec<(&Node, Result<Segment>)>>();

        let body = self
            .from_sub_segments(0, rnode.as_node().end_byte(), pitems)?
            .body;

        Ok(Snippet { body })
    }

    fn from_node(&self, node: &Node) -> Result<Segment, anyhow::Error> {
        match node.kind() {
            NodeType::Ellipsis | NodeType::EllipsisMetavariable(_) => Err(anyhow!(
                "cannot use ellipsis operator inside the transformation query"
            )),
            NodeType::Metavariable(mid) => self.from_metavariable(node, &mid.0),
            _ if (node.children.len() == 0 || T::is_leaf_like(node)) => self.from_leaf(node),
            _ => self.from_intermediate_node(node),
        }
    }

    fn from_metavariable(
        &self,
        node: &Node,
        variable_name: &str,
    ) -> Result<Segment, anyhow::Error> {
        let id = MetavariableId(variable_name.into());
        let value = self
            .item
            .capture_of(&id)
            .and_then(|x| match x {
                CaptureItem::Empty => None,
                _ => Some(x.as_str()),
            })
            .ok_or(SnippetBuilderError::MetavariableUnavailable {
                id: id.0,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            })?;

        Ok(Segment {
            body: value.into(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    fn from_leaf(&self, node: &Node) -> Result<Segment, anyhow::Error> {
        Ok(Segment {
            body: node.as_str().into(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    fn from_intermediate_node(&self, node: &Node) -> Result<Segment, anyhow::Error> {
        let children = node
            .children
            .iter()
            .map(|child| (child, self.from_node(child)))
            .collect::<Vec<(&Node, Result<Segment>)>>();

        self.from_sub_segments(node.start_byte(), node.end_byte(), children)
    }

    /// `from_patched_items` generates TransformedSegment for the range [start_byte, end_byte) by combining multiple and ordered TransformedSegments in the same range.
    fn from_sub_segments(
        &self,
        start_byte: usize,
        end_byte: usize,
        subitems: Vec<(&Node, Result<Segment>)>,
    ) -> Result<Segment, anyhow::Error> {
        let mut body: String = "".into();

        let mut virtual_end_byte: usize = start_byte;
        let mut end_byte_for_last_real_node: Option<usize> = None;

        let mut children_iter = subitems.into_iter();
        let mut next_ = children_iter.next();

        while let Some((n, child)) = next_ {
            match child {
                Err(ref e)
                    if matches!(
                        e.downcast_ref::<SnippetBuilderError>(),
                        Some(&SnippetBuilderError::MetavariableUnavailable { .. })
                    ) =>
                {
                    // `child` is a TransformedSegment whose content is empty or undefined metavariable.
                    // Shisho allows this, and it treats it as if the node does not exist.
                    //
                    // Suppose the following RewriteOption where `BAR` is (1) captured but empty or (2) undefined:
                    //
                    // ```
                    // :[FOO]
                    // :[BAR]
                    // :[PIYO]
                    // ```
                    //
                    // Shisho treats the RewriteOption as same as the following RewriteOption:
                    //
                    // ```
                    // :[FOO]
                    // :[PIYO]
                    // ```
                    let mut glue = "".to_string();
                    if let Some(e) = end_byte_for_last_real_node {
                        glue = self.autofix.pattern.string_between(e, n.start_byte())?;
                        end_byte_for_last_real_node = None;
                    }

                    next_ = children_iter.next();
                    if let Some((ref n, _)) = next_ {
                        body += glue.as_str();
                        virtual_end_byte = n.start_byte();
                    } else {
                        virtual_end_byte = end_byte;
                    }
                }
                Err(e) => {
                    // an undesired error happened on TransformedSegment generation.
                    return Err(e);
                }
                Ok(child) => {
                    // `child` is a normal TransformedSegment.

                    // (1) handle between the previous TransformedSegment and `child`.
                    let pre_glue = self
                        .autofix
                        .pattern
                        .string_between(virtual_end_byte, child.start_byte)?;
                    body += pre_glue.as_str();

                    // (2) handle `child` itself
                    body += child.body.as_str();

                    virtual_end_byte = child.end_byte;
                    end_byte_for_last_real_node = Some(child.end_byte);
                    next_ = children_iter.next();
                }
            }
        }

        let post_glue = self
            .autofix
            .pattern
            .string_between(virtual_end_byte, end_byte)?;
        body += post_glue.as_str();

        Ok(Segment {
            body,
            start_byte,
            end_byte,
        })
    }
}
