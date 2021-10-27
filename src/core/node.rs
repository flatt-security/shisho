use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use super::{query::MetavariableId, source::NormalizedSource};

const SHISHO_NODE_METAVARIABLE_NAME: &str = "shisho_metavariable_name";
const SHISHO_NODE_METAVARIABLE: &str = "shisho_metavariable";
const SHISHO_NODE_ELLIPSIS_METAVARIABLE: &str = "shisho_ellipsis_metavariable";
const SHISHO_NODE_ELLIPSIS: &str = "shisho_ellipsis";

#[derive(Debug, Clone, PartialEq)]
pub struct Node<'tree> {
    inner: tree_sitter::Node<'tree>,

    pub children: Vec<Node<'tree>>,
    pub(crate) source: &'tree NormalizedSource,
}

#[derive(Debug, PartialEq, Clone)]
pub enum NodeType {
    Metavariable(MetavariableId),
    EllipsisMetavariable(MetavariableId),
    Ellipsis,
    Normal(&'static str),
}

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

impl<'tree> Node<'tree> {
    pub fn from_tsnode(tsnode: tree_sitter::Node<'tree>, source: &'tree NormalizedSource) -> Self {
        let children: Vec<Self> = tsnode
            .children(&mut tsnode.walk())
            .map(|c| Self::from_tsnode(c, source))
            .collect();
        Node {
            inner: tsnode,
            children,
            source,
        }
    }
}

#[derive(Debug)]
pub struct RootNode<'tree>(Node<'tree>);

impl<'tree> RootNode<'tree> {
    pub fn from_tstree(tstree: &'tree tree_sitter::Tree, source: &'tree NormalizedSource) -> Self {
        let tsnode = tstree.root_node();
        let children: Vec<Node<'tree>> = tsnode
            .children(&mut tsnode.walk())
            .map(|c| Node::from_tsnode(c, source))
            .collect();
        RootNode::new(Node {
            inner: tsnode,
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

    pub fn source(&self) -> &NormalizedSource {
        &self.as_node().source
    }
}

impl<'tree> From<RootNode<'tree>> for Node<'tree> {
    fn from(r: RootNode<'tree>) -> Self {
        r.0
    }
}

impl<'tree> From<&'tree RootNode<'tree>> for &'tree Node<'tree> {
    fn from(r: &'tree RootNode<'tree>) -> Self {
        &r.0
    }
}

pub trait NodeLike
where
    Self: Sized + Clone + std::fmt::Debug,
{
    fn kind(&self) -> NodeType;
    fn children<'a>(&'a self) -> Vec<&'a Self>;

    fn start_byte(&self) -> usize;
    fn end_byte(&self) -> usize;
    fn start_position(&self) -> tree_sitter::Point;
    fn end_position(&self) -> tree_sitter::Point;

    fn as_cow(&self) -> Cow<'_, str>;
    fn with_source<'a, F, Output>(&'a self, callback: F) -> Output
    where
        F: Fn(&NormalizedSource) -> Output,
        Output: 'a;
}

fn get_metavariable_id<'a>(node: &'a Node<'_>) -> Cow<'a, str> {
    node.children
        .iter()
        .find(|child| child.kind() == NodeType::Normal(SHISHO_NODE_METAVARIABLE_NAME))
        .map(|child| child.as_cow())
        .expect(format!("no {} found", SHISHO_NODE_METAVARIABLE_NAME).as_str())
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
        std::borrow::Cow::Borrowed(
            self.source
                .as_str_between(self.start_byte(), self.end_byte())
                .unwrap(),
        )
    }

    fn children<'a>(&'a self) -> Vec<&'a Self> {
        self.children.iter().collect()
    }

    fn with_source<'a, F, Output>(&'a self, callback: F) -> Output
    where
        F: Fn(&NormalizedSource) -> Output,
        Output: 'a,
    {
        callback(self.source.into())
    }
}
