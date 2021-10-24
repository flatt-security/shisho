use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use super::query::MetavariableId;

const SHISHO_NODE_METAVARIABLE_NAME: &str = "shisho_metavariable_name";
const SHISHO_NODE_METAVARIABLE: &str = "shisho_metavariable";
const SHISHO_NODE_ELLIPSIS_METAVARIABLE: &str = "shisho_ellipsis_metavariable";
const SHISHO_NODE_ELLIPSIS: &str = "shisho_ellipsis";

/// `Range` describes a range over a source code in a same manner as [Language Server Protocol](https://microsoft.github.io/language-server-protocol/specifications/specification-current/#range).
#[derive(Debug, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node<'tree> {
    inner: tree_sitter::Node<'tree>,
    with_extra_newline: bool,

    pub children: Vec<Node<'tree>>,
    pub(crate) source: &'tree [u8],
}

#[derive(Debug, PartialEq, Clone)]
pub enum NodeType {
    Metavariable(MetavariableId),
    EllipsisMetavariable(MetavariableId),
    Ellipsis,
    Normal(&'static str),
}

fn get_metavariable_id<'a>(node: &'a Node<'_>) -> Cow<'a, str> {
    node.children
        .iter()
        .find(|child| child.kind() == NodeType::Normal(SHISHO_NODE_METAVARIABLE_NAME))
        .map(|child| child.as_cow())
        .expect(
            format!(
                "{} did not have {}",
                SHISHO_NODE_ELLIPSIS_METAVARIABLE, SHISHO_NODE_METAVARIABLE_NAME
            )
            .as_str(),
        )
}

pub trait NodeLike
where
    Self: Sized + Clone,
{
    fn kind(&self) -> NodeType;
    fn with_extra_newline(&self) -> bool;
    fn children<'a>(&'a self) -> Vec<&'a Self>;

    fn start_byte(&self) -> usize;
    fn end_byte(&self) -> usize;
    fn start_position(&self) -> tree_sitter::Point;
    fn end_position(&self) -> tree_sitter::Point;

    fn as_cow(&self) -> Cow<'_, str>;
    fn with_source<'a, F, Output>(&'a self, callback: F) -> Output
    where
        F: Fn(&[u8]) -> Output,
        Output: 'a;
}

impl<'tree> NodeLike for Node<'tree> {
    fn kind(&self) -> NodeType {
        match self.inner.kind() {
            s if s == SHISHO_NODE_METAVARIABLE => {
                NodeType::Metavariable(MetavariableId(get_metavariable_id(self).to_string()))
            }
            s if s == SHISHO_NODE_ELLIPSIS => NodeType::Ellipsis,
            s if s == SHISHO_NODE_ELLIPSIS_METAVARIABLE => NodeType::EllipsisMetavariable(
                MetavariableId(get_metavariable_id(self).to_string()),
            ),
            s => NodeType::Normal(s),
        }
    }

    fn with_extra_newline(&self) -> bool {
        self.with_extra_newline
    }

    fn start_byte(&self) -> usize {
        self.inner.start_byte()
    }

    fn end_byte(&self) -> usize {
        self.inner.end_byte()
    }

    fn start_position(&self) -> tree_sitter::Point {
        self.inner.start_position()
    }

    fn end_position(&self) -> tree_sitter::Point {
        self.inner.end_position()
    }

    fn as_cow(&self) -> Cow<'_, str> {
        let last = if self.with_extra_newline {
            self.end_byte() - 1
        } else {
            self.end_byte()
        };
        std::borrow::Cow::Borrowed(
            core::str::from_utf8(&self.source[self.start_byte()..last]).unwrap(),
        )
    }

    fn children<'a>(&'a self) -> Vec<&'a Self> {
        self.children.iter().collect()
    }

    fn with_source<'a, F, Output>(&'a self, callback: F) -> Output
    where
        F: Fn(&[u8]) -> Output,
        Output: 'a,
    {
        callback(self.source)
    }
}

impl<'tree> Node<'tree> {
    pub fn from_tsnode(
        tsnode: tree_sitter::Node<'tree>,
        source: &'tree [u8],
        extra_newline_byte: Option<usize>,
    ) -> Self {
        let children: Vec<Self> = tsnode
            .children(&mut tsnode.walk())
            .map(|c| Self::from_tsnode(c, source, extra_newline_byte))
            .collect();
        Node {
            inner: tsnode,
            with_extra_newline: extra_newline_byte == Some(tsnode.end_byte() - 1),
            children,
            source,
        }
    }
}

#[derive(Debug)]
pub struct RootNode<'tree>(Node<'tree>);

impl<'tree> RootNode<'tree> {
    pub fn from_tstree(
        tstree: &'tree tree_sitter::Tree,
        source: &'tree [u8],
        with_extra_newline: bool,
    ) -> Self {
        let tsnode = tstree.root_node();
        let children: Vec<Node<'tree>> = tsnode
            .children(&mut tsnode.walk())
            .map(|c| {
                Node::from_tsnode(
                    c,
                    source,
                    if with_extra_newline {
                        Some(source.len() - 1)
                    } else {
                        None
                    },
                )
            })
            .collect();
        RootNode::new(Node {
            inner: tsnode,
            with_extra_newline,
            children,
            source,
        })
    }
}

impl<'tree> RootNode<'tree> {
    pub fn new(node: Node<'tree>) -> Self {
        Self(node)
    }

    pub fn as_node(&self) -> &Node<'tree> {
        &self.0
    }
}

impl<'tree> From<RootNode<'tree>> for Node<'tree> {
    fn from(r: RootNode<'tree>) -> Self {
        r.0
    }
}
