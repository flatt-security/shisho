use std::marker::PhantomData;

use crate::core::{
    language::Queryable,
    matcher::CaptureItem,
    matcher::MatchedItem,
    node::NodeType,
    node::{Node, NodeLike},
    pattern::Pattern,
    query::MetavariableId,
    ruleset::{
        constraint::PatternWithConstraints,
        filter::{RewriteFilter, RewriteFilterPredicate},
    },
    tree::RefTreeView,
};
use anyhow::{anyhow, Result};
use regex::Captures;
use thiserror::Error;

use super::{node::RewritableNode, tree::NormalizedRewritableTree};

pub struct SnippetBuilder<'tree, T>
where
    T: Queryable,
{
    replace_target: &'tree MatchedItem<'tree, Node<'tree>>,
    replace_with: NormalizedRewritableTree<'tree, T>,

    _marker: PhantomData<T>,
}

impl<'tree, T> SnippetBuilder<'tree, T>
where
    T: Queryable,
{
    pub fn new(
        replace_target: &'tree MatchedItem<'tree, Node<'tree>>,
        replace_with: NormalizedRewritableTree<'tree, T>,
    ) -> Self {
        Self {
            replace_target,
            replace_with,
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

/// Tree Editor
impl<'tree, T> SnippetBuilder<'tree, T>
where
    T: Queryable,
{
    pub fn replace(
        &mut self,
        target: &MetavariableId,

        pwc: &PatternWithConstraints<T>,
        with_pattern: &Pattern<T>,
    ) -> Result<()> {
        // let rtv = RefTreeView::from(from capture);

        // let matches = rtv
        //     .matches(&pwc.as_query())
        //     .collect::<Result<Vec<MatchedItem<RewritableNode>>>>()?;

        // todo!("not implemented yet")

        Ok(())
    }

    pub fn apply_filter(&mut self, filter: &RewriteFilter<T>) -> Result<&mut Self> {
        match &filter.predicate {
            RewriteFilterPredicate::ReplaceWithQuery((pwcs, to)) => {
                for pwc in pwcs {
                    self.replace(&filter.target, pwc, to)?;
                }
            }
        }

        Ok(self)
    }

    pub fn apply_filters(&mut self, filters: &Vec<RewriteFilter<T>>) -> Result<&mut Self> {
        for filter in filters {
            self.apply_filter(filter)?;
        }

        Ok(self)
    }
}

/// Snippet Constructor
impl<'tree, T> SnippetBuilder<'tree, T>
where
    T: Queryable,
{
    pub fn build(&self) -> Result<Snippet> {
        let pitems: Vec<(&RewritableNode, Result<Segment>)> = vec![(
            &self.replace_with.root,
            self.build_from_node(&self.replace_with.root),
        )];

        let body = self
            .build_segment_from_segments(0, self.replace_with.root.end_byte(), pitems)?
            .body;

        Ok(Snippet { body })
    }

    fn build_from_node(&self, node: &RewritableNode) -> Result<Segment, anyhow::Error> {
        match node.kind() {
            NodeType::Ellipsis => Err(anyhow!(
                "cannot use ellipsis operator inside the transformation query"
            )),
            NodeType::Metavariable(mid) | NodeType::EllipsisMetavariable(mid) => {
                self.build_from_metavariable(node, &mid.0)
            }
            _ if (node.children.is_empty() || T::is_leaf_like(node)) => self.build_from_leaf(node),
            _ => self.build_from_intermediate_node(node),
        }
    }

    fn build_from_metavariable(
        &self,
        node: &RewritableNode,
        variable_name: &str,
    ) -> Result<Segment, anyhow::Error> {
        let id = MetavariableId(variable_name.into());
        let value = self
            .replace_target
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

    fn build_from_leaf(&self, node: &RewritableNode) -> Result<Segment, anyhow::Error> {
        assert!(node.children.is_empty() || T::is_leaf_like(node));

        if T::is_string_literal(node) {
            self.build_from_string_leaf(node)
        } else {
            Ok(Segment {
                body: node.as_cow().into(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            })
        }
    }

    fn build_from_intermediate_node(
        &self,
        node: &RewritableNode,
    ) -> Result<Segment, anyhow::Error> {
        let children = node
            .children
            .iter()
            .map(|child| (child, self.build_from_node(child)))
            .collect::<Vec<(&RewritableNode, Result<Segment>)>>();

        self.build_segment_from_segments(node.start_byte(), node.end_byte(), children)
    }

    /// `from_patched_items` generates TransformedSegment for the range [start_byte, end_byte) by combining multiple and ordered TransformedSegments in the same range.
    fn build_segment_from_segments(
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
                    let mut glue = String::default();
                    if let Some(e) = end_byte_for_last_real_node {
                        glue = self.to_string_between(e, n.start_byte())?;
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
                    let pre_glue = self.to_string_between(virtual_end_byte, child.start_byte)?;
                    body += pre_glue.as_str();

                    // (2) handle `child` itself
                    body += child.body.as_str();

                    virtual_end_byte = child.end_byte;
                    end_byte_for_last_real_node = Some(child.end_byte);
                    next_ = children_iter.next();
                }
            }
        }

        let post_glue = self.to_string_between(virtual_end_byte, end_byte)?;
        body += post_glue.as_str();

        Ok(Segment {
            body,
            start_byte,
            end_byte,
        })
    }

    fn build_from_string_leaf(&self, node: &RewritableNode) -> Result<Segment> {
        assert!((node.children.is_empty() || T::is_leaf_like(node)) && T::is_string_literal(node));

        let body = node.as_cow().to_string();
        let r = regex::Regex::new(r":\[(\.\.\.)?(?P<name>[A-Z_][A-Z_0-9]*)\]").unwrap();
        let body = r.replace_all(body.as_str(), |caps: &Captures| {
            let name = caps.name("name").unwrap().as_str();
            self.build_from_metavariable(node, name)
                .map(|x| x.body)
                .unwrap_or_default()
        });
        Ok(Segment {
            body: body.into(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    #[inline]
    fn to_string_between(&self, start: usize, end: usize) -> Result<String> {
        let source = self.replace_with.source;
        source.as_str_between(start, end).map(|x| x.to_string())
    }
}
