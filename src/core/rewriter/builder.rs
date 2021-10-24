use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use crate::core::{
    language::Queryable,
    matcher::CaptureItem,
    matcher::MatchedItem,
    node::NodeType,
    node::{Node, NodeLike},
    query::MetavariableId,
};
use anyhow::{anyhow, Result};
use thiserror::Error;

use super::{node::RewritableNode, RewriteOption};

pub struct SnippetBuilder<'pattern, T>
where
    T: Queryable,
{
    root_node: RewritableNode,
    source: Rc<RefCell<Vec<u8>>>,
    with_extra_newline: bool,

    item: &'pattern MatchedItem<'pattern, Node<'pattern>>,

    _marker: PhantomData<T>,
}

impl<'pattern, T> SnippetBuilder<'pattern, T>
where
    T: Queryable,
{
    pub fn new(
        autofix: RewriteOption<'pattern, T>,
        item: &'pattern MatchedItem<'pattern, Node<'pattern>>,
    ) -> Self {
        let source = autofix.pattern.source.clone();
        let source = Rc::new(RefCell::new(source));

        let rnode: Node = autofix.root_node.into();
        Self {
            root_node: RewritableNode::from_node(rnode, source.clone()),
            source,
            with_extra_newline: autofix.pattern.with_extra_newline,

            item,

            _marker: PhantomData,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Segment {
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
    pub fn build(&self) -> Result<Snippet, anyhow::Error> {
        let pitems: Vec<(&RewritableNode, Result<Segment>)> =
            vec![(&self.root_node, self.from_node(&self.root_node))];

        let body = self
            .from_sub_segments(0, self.root_node.end_byte(), pitems)?
            .body;

        Ok(Snippet { body })
    }

    fn from_node(&self, node: &RewritableNode) -> Result<Segment, anyhow::Error> {
        match node.kind() {
            NodeType::Ellipsis => Err(anyhow!(
                "cannot use ellipsis operator inside the transformation query"
            )),
            NodeType::Metavariable(mid) | NodeType::EllipsisMetavariable(mid) => {
                self.from_metavariable(node, &mid.0)
            }
            _ if (node.children.is_empty() || T::is_leaf_like(node)) => self.from_leaf(node),
            _ => self.from_intermediate_node(node),
        }
    }

    pub(crate) fn from_metavariable(
        &self,
        node: &RewritableNode,
        variable_name: &str,
    ) -> Result<Segment, anyhow::Error> {
        let id = MetavariableId(variable_name.into());
        let value = self
            .item
            .capture_of(&id)
            .and_then(|x| match x {
                CaptureItem::Empty => None,
                _ => Some(x.to_string()),
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

    fn from_leaf(&self, node: &RewritableNode) -> Result<Segment, anyhow::Error> {
        if T::is_string_literal(node) {
            self.from_string_leaf(node)
        } else {
            Ok(Segment {
                body: node.as_cow().into(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            })
        }
    }

    fn from_intermediate_node(&self, node: &RewritableNode) -> Result<Segment, anyhow::Error> {
        let children = node
            .children
            .iter()
            .map(|child| (child, self.from_node(child)))
            .collect::<Vec<(&RewritableNode, Result<Segment>)>>();

        self.from_sub_segments(node.start_byte(), node.end_byte(), children)
    }

    /// `from_patched_items` generates TransformedSegment for the range [start_byte, end_byte) by combining multiple and ordered TransformedSegments in the same range.
    fn from_sub_segments(
        &self,
        start_byte: usize,
        end_byte: usize,
        subitems: Vec<(&RewritableNode, Result<Segment>)>,
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
                        glue = self.string_between(e, n.start_byte())?;
                        end_byte_for_last_real_node = None;
                    }

                    next_ = children_iter.next();
                    if let Some((n, _)) = next_ {
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
                    let pre_glue = self.string_between(virtual_end_byte, child.start_byte)?;
                    body += pre_glue.as_str();

                    // (2) handle `child` itself
                    body += child.body.as_str();

                    virtual_end_byte = child.end_byte;
                    end_byte_for_last_real_node = Some(child.end_byte);
                    next_ = children_iter.next();
                }
            }
        }

        let post_glue = self.string_between(virtual_end_byte, end_byte)?;
        body += post_glue.as_str();

        Ok(Segment {
            body,
            start_byte,
            end_byte,
        })
    }

    #[inline]
    pub fn string_between(&self, start: usize, end: usize) -> Result<String> {
        let source = self.source.borrow();

        let start = if source.len() == start && self.with_extra_newline {
            start - 1
        } else {
            start
        };
        let end = if source.len() == end && self.with_extra_newline {
            end - 1
        } else {
            end
        };
        Ok(String::from_utf8(source[start..end].to_vec())?)
    }
}
