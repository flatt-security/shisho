use id_arena::{Arena, Id};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, ops::Sub};

use super::{query::MetavariableId, source::NormalizedSource, tree::RootedTreeLike};

const SHISHO_NODE_METAVARIABLE_NAME: &str = "shisho_metavariable_name";
const SHISHO_NODE_METAVARIABLE: &str = "shisho_metavariable";
const SHISHO_NODE_ELLIPSIS_METAVARIABLE: &str = "shisho_ellipsis_metavariable";
const SHISHO_NODE_ELLIPSIS: &str = "shisho_ellipsis";

pub type NodeLikeId<'tree, N: NodeLike<'tree>> = Id<N>;
pub type NodeLikeArena<'tree, N: NodeLike<'tree>> = Arena<N>;

#[derive(Debug, Clone, PartialEq)]
pub struct CSTNode<'tree> {
    kind: NodeType,

    start_byte: usize,
    end_byte: usize,

    start_position: TSPoint,
    end_position: TSPoint,

    pub children: Vec<NodeLikeId<'tree, Self>>,
    pub(crate) source: &'tree NormalizedSource,
}

#[derive(Debug, PartialEq, Clone)]
pub enum NodeType {
    Metavariable(MetavariableId),
    EllipsisMetavariable(MetavariableId),
    Ellipsis,
    Normal(&'static str),
    Unifier,
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

pub type CSTNodeId<'tree> = NodeLikeId<'tree, CSTNode<'tree>>;
pub type CSTNodeArena<'tree> = NodeLikeArena<'tree, CSTNode<'tree>>;
pub type CSTNodeRefWithId<'tree> = NodeLikeRefWithId<'tree, CSTNode<'tree>>;

impl<'tree> CSTNode<'tree> {
    pub fn from_tsnode<'node>(
        tsnode: tree_sitter::Node<'node>,
        source: &'tree NormalizedSource,
        arena: &mut CSTNodeArena<'tree>,
    ) -> NodeLikeId<'tree, Self>
    where
        'tree: 'node,
    {
        let mut children: Vec<CSTNodeId<'tree>> = vec![];
        for c in tsnode.children(&mut tsnode.walk()) {
            children.push(Self::from_tsnode(c, source, arena));
        }

        let kind = match tsnode.kind() {
            s if s == SHISHO_NODE_METAVARIABLE => {
                NodeType::Metavariable(MetavariableId(get_metavariable_id(&children, arena)))
            }
            s if s == SHISHO_NODE_ELLIPSIS => NodeType::Ellipsis,
            s if s == SHISHO_NODE_ELLIPSIS_METAVARIABLE => NodeType::EllipsisMetavariable(
                MetavariableId(get_metavariable_id(&children, arena)),
            ),
            s => NodeType::Normal(s),
        };

        arena.alloc(CSTNode {
            kind,

            start_byte: tsnode.start_byte(),
            end_byte: tsnode.end_byte(),

            start_position: tsnode.start_position().into(),
            end_position: tsnode.end_position().into(),

            children,
            source,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TSPoint {
    pub row: usize,
    pub column: usize,
}

impl From<tree_sitter::Point> for TSPoint {
    fn from(p: tree_sitter::Point) -> Self {
        Self {
            row: p.row,
            column: p.column,
        }
    }
}

impl Sub for TSPoint {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            row: self.row - rhs.row,
            column: if self.row == rhs.row {
                self.column - rhs.column
            } else {
                self.column
            },
        }
    }
}

pub trait NodeLike<'tree>
where
    Self: Sized + Clone + std::fmt::Debug,
{
    fn kind(&self) -> NodeType;
    fn children<V: RootedTreeLike<'tree, Self>>(&'tree self, tview: &'tree V) -> Vec<&'tree Self>;
    fn indexed_children<V: RootedTreeLike<'tree, Self>>(
        &'tree self,
        tview: &'tree V,
    ) -> Vec<NodeLikeRefWithId<'tree, Self>>;

    fn start_byte(&self) -> usize;
    fn end_byte(&self) -> usize;
    fn start_position(&self) -> TSPoint;
    fn end_position(&self) -> TSPoint;

    fn as_cow(&self) -> Cow<'_, str>;
    fn with_source<'a, F, Output>(&'a self, callback: F) -> Output
    where
        F: Fn(&NormalizedSource) -> Output,
        Output: 'a;
}

fn get_metavariable_id<'a>(
    children: &'a Vec<NodeLikeId<'_, CSTNode>>,
    arena: &'a Arena<CSTNode>,
) -> String {
    children
        .iter()
        .map(|x| arena.get(*x).unwrap())
        .find(|child| child.kind() == NodeType::Normal(SHISHO_NODE_METAVARIABLE_NAME))
        .map(|child| child.as_cow())
        .map(|child| child.to_string())
        .expect(format!("no {} found", SHISHO_NODE_METAVARIABLE_NAME).as_str())
}

impl<'tree> NodeLike<'tree> for CSTNode<'tree> {
    fn kind(&self) -> NodeType {
        self.kind.clone()
    }

    fn start_byte(&self) -> usize {
        self.start_byte
    }

    fn end_byte(&self) -> usize {
        self.end_byte
    }

    fn start_position(&self) -> TSPoint {
        self.start_position
    }

    fn end_position(&self) -> TSPoint {
        self.end_position
    }

    fn as_cow(&self) -> Cow<'_, str> {
        std::borrow::Cow::Borrowed(
            self.source
                .as_str_between(self.start_byte(), self.end_byte())
                .unwrap(),
        )
    }

    fn children<V: RootedTreeLike<'tree, Self>>(&'tree self, tview: &'tree V) -> Vec<&'tree Self> {
        self.children
            .iter()
            .map(|x| tview.get(*x).unwrap())
            .collect()
    }

    fn indexed_children<V: RootedTreeLike<'tree, Self>>(
        &'tree self,
        tview: &'tree V,
    ) -> Vec<NodeLikeRefWithId<'tree, Self>> {
        self.children
            .iter()
            .map(|x| NodeLikeRefWithId {
                id: *x,
                node: tview.get(*x).unwrap(),
            })
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

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct NodeLikeRefWithId<'tree, N: NodeLike<'tree>> {
    pub id: NodeLikeId<'tree, N>,
    pub node: &'tree N,
}

impl<'tree, N: NodeLike<'tree>> NodeLikeRefWithId<'tree, N> {
    pub fn kind(&self) -> NodeType {
        self.node.kind()
    }

    pub fn start_byte(&self) -> usize {
        self.node.start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.node.end_byte()
    }

    pub fn start_position(&self) -> TSPoint {
        self.node.start_position()
    }

    pub fn end_position(&self) -> TSPoint {
        self.node.end_position()
    }

    pub fn as_cow(&self) -> Cow<'_, str> {
        self.node.as_cow()
    }

    pub fn children<V: RootedTreeLike<'tree, N>>(&self, tview: &'tree V) -> Vec<Self> {
        self.node.indexed_children(tview)
    }

    pub fn with_source<'a, F, Output>(&'a self, callback: F) -> Output
    where
        F: Fn(&NormalizedSource) -> Output,
        Output: 'a,
    {
        self.node.with_source(callback)
    }
}
