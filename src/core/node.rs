use id_arena::{Arena, Id};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use super::{query::MetavariableId, source::NormalizedSource};

const SHISHO_NODE_METAVARIABLE_NAME: &str = "shisho_metavariable_name";
const SHISHO_NODE_METAVARIABLE: &str = "shisho_metavariable";
const SHISHO_NODE_ELLIPSIS_METAVARIABLE: &str = "shisho_ellipsis_metavariable";
const SHISHO_NODE_ELLIPSIS: &str = "shisho_ellipsis";

type NodeId<'tree> = Id<Node<'tree>>;

#[derive(Debug, Clone, PartialEq)]
pub struct Node<'tree> {
    kind: NodeType,

    start_byte: usize,
    end_byte: usize,

    start_position: tree_sitter::Point,
    end_position: tree_sitter::Point,

    pub children: Vec<NodeId<'tree>>,
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
    pub fn from_tsnode(
        tsnode: tree_sitter::Node<'tree>,
        source: &'tree NormalizedSource,
        arena: &'tree mut Arena<Node>,
    ) -> NodeId<'tree> {
        let children: Vec<NodeId> = tsnode
            .children(&mut tsnode.walk())
            .map(|c| Self::from_tsnode(c, source, arena))
            .collect();

        let kind = match tsnode.kind() {
            s if s == SHISHO_NODE_METAVARIABLE => NodeType::Metavariable(MetavariableId(
                get_metavariable_id(&children, arena).to_string(),
            )),
            s if s == SHISHO_NODE_ELLIPSIS => NodeType::Ellipsis,
            s if s == SHISHO_NODE_ELLIPSIS_METAVARIABLE => NodeType::EllipsisMetavariable(
                MetavariableId(get_metavariable_id(&children, arena).to_string()),
            ),
            s => NodeType::Normal(s),
        };

        arena.alloc(Node {
            kind,

            start_byte: tsnode.start_byte(),
            end_byte: tsnode.end_byte(),

            start_position: tsnode.start_position(),
            end_position: tsnode.end_position(),

            children,
            source,
        })
    }
}

#[derive(Debug)]
pub struct RootNode<'tree>(Node<'tree>);

impl<'tree> RootNode<'tree> {
    pub fn from_tstree(tstree: &'tree tree_sitter::Tree, source: &'tree NormalizedSource) -> Self {
        let tsnode = tstree.root_node();
        let node = Node::from_tsnode(tsnode, source);
        RootNode::new(node)
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

pub trait NodeLike<'tree>
where
    Self: Sized + Clone + std::fmt::Debug,
{
    fn kind(&self) -> NodeType;
    fn children(&'tree self, arena: &'tree Arena<Node<'tree>>) -> Vec<&'tree Self>;

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

fn get_metavariable_id<'a>(children: &'a Vec<NodeId<'_>>, arena: &'a Arena<Node>) -> Cow<'a, str> {
    children
        .iter()
        .map(|x| arena.get(*x).unwrap())
        .find(|child| child.kind() == NodeType::Normal(SHISHO_NODE_METAVARIABLE_NAME))
        .map(|child| child.as_cow())
        .expect(format!("no {} found", SHISHO_NODE_METAVARIABLE_NAME).as_str())
}

impl<'tree> NodeLike<'tree> for Node<'tree> {
    fn kind(&self) -> NodeType {
        self.kind
    }

    fn start_byte(&self) -> usize {
        self.start_byte
    }

    fn end_byte(&self) -> usize {
        self.end_byte
    }

    fn start_position(&self) -> tree_sitter::Point {
        self.start_position
    }

    fn end_position(&self) -> tree_sitter::Point {
        self.end_position
    }

    fn as_cow(&self) -> Cow<'_, str> {
        std::borrow::Cow::Borrowed(
            self.source
                .as_str_between(self.start_byte(), self.end_byte())
                .unwrap(),
        )
    }

    fn children(&'tree self, arena: &'tree Arena<Node<'tree>>) -> Vec<&'tree Self> {
        self.children
            .iter()
            .map(|x| arena.get(*x).unwrap())
            .collect()
    }

    fn with_source<'a, F, Output>(&'a self, callback: F) -> Output
    where
        F: Fn(&NormalizedSource) -> Output,
        Output: 'a,
    {
        callback(self.source.into())
    }
}
