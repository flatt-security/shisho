use std::{collections::HashMap, convert::TryFrom};

use crate::core::language::Queryable;
use serde::{Deserialize, Serialize};

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

    pub(crate) source: &'tree [u8],
    pub children: Vec<Node<'tree>>,
}

#[derive(Debug, PartialEq)]
pub enum NodeType {
    Metavariable(MetavariableId),
    EllipsisMetavariable(MetavariableId),
    Ellipsis,
    Normal(&'static str),
}

fn get_metavariable_id<'a>(node: &'a Node<'_>) -> &'a str {
    node.children
        .iter()
        .find(|child| child.kind() == NodeType::Normal(SHISHO_NODE_METAVARIABLE_NAME))
        .map(|child| child.as_str())
        .expect(
            format!(
                "{} did not have {}",
                SHISHO_NODE_ELLIPSIS_METAVARIABLE, SHISHO_NODE_METAVARIABLE_NAME
            )
            .as_str(),
        )
}

impl<'tree> Node<'tree> {
    pub fn kind(&self) -> NodeType {
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

    pub fn start_byte(&self) -> usize {
        self.inner.start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.inner.end_byte()
    }

    pub fn start_position(&self) -> tree_sitter::Point {
        self.inner.start_position()
    }

    pub fn end_position(&self) -> tree_sitter::Point {
        self.inner.end_position()
    }

    pub fn as_str(&self) -> &'tree str {
        let last = if self.with_extra_newline {
            self.end_byte() - 1
        } else {
            self.end_byte()
        };
        core::str::from_utf8(&self.source[self.start_byte()..last]).unwrap()
    }

    pub fn is_named(&self) -> bool {
        self.inner.is_named()
    }

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

#[derive(Debug, Clone, PartialEq)]
pub struct ConsecutiveNodes<'tree> {
    inner: Vec<&'tree Node<'tree>>,
    source: &'tree [u8],
}

impl<'tree> TryFrom<Vec<&'tree Node<'tree>>> for ConsecutiveNodes<'tree> {
    type Error = anyhow::Error;
    fn try_from(inner: Vec<&'tree Node<'tree>>) -> Result<Self, Self::Error> {
        // TODO (y0n3uchy): check all capture items are consecutive
        if inner.is_empty() {
            Err(anyhow::anyhow!(
                "internal error; ConsecutiveNodes was generated from empty vec."
            ))
        } else {
            let source = inner.get(0).unwrap().source;
            Ok(ConsecutiveNodes { inner, source })
        }
    }
}

impl<'tree> TryFrom<Vec<ConsecutiveNodes<'tree>>> for ConsecutiveNodes<'tree> {
    type Error = anyhow::Error;

    fn try_from(cns: Vec<ConsecutiveNodes<'tree>>) -> Result<Self, Self::Error> {
        // TODO (y0n3uchy): check all capture items are consecutive
        if cns.is_empty() {
            Err(anyhow::anyhow!(
                "internal error; ConsecutiveNodes was generated from empty vec."
            ))
        } else {
            let source = cns.get(0).unwrap().source;
            Ok(ConsecutiveNodes {
                inner: cns.into_iter().map(|cn| cn.inner).flatten().collect(),
                source,
            })
        }
    }
}

impl<'tree> ConsecutiveNodes<'tree> {
    pub fn as_vec(&self) -> &Vec<&'tree Node<'tree>> {
        &self.inner
    }

    pub fn push(&mut self, n: &'tree Node<'tree>) {
        self.inner.push(n)
    }

    pub fn start_position(&self) -> tree_sitter::Point {
        self.as_vec().first().unwrap().start_position()
    }

    pub fn end_position(&self) -> tree_sitter::Point {
        self.as_vec().last().unwrap().end_position()
    }

    pub fn range<T: Queryable>(&self) -> Range {
        Range {
            start: T::range(self.as_vec().first().unwrap()).start,
            end: T::range(self.as_vec().last().unwrap()).end,
        }
    }

    pub fn start_byte(&self) -> usize {
        self.as_vec().first().unwrap().start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.as_vec().last().unwrap().end_byte()
    }

    pub fn len(&self) -> usize {
        self.as_vec().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn with_extra_newline(&self) -> bool {
        self.as_vec().last().unwrap().with_extra_newline
    }

    #[inline]
    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        let last = if self.with_extra_newline() {
            self.end_byte() - 1
        } else {
            self.end_byte()
        };
        core::str::from_utf8(&self.source[self.start_byte()..last])
    }
}

pub struct Path {
    components: Vec<PathComponent>,
}

pub enum PathComponent {
    Index(usize),
    KeyName(String),
}

pub struct BlockId(usize);

pub type BlockMap = HashMap<BlockId, Block>;

pub struct Block {
    id: BlockId,

    /// type of this block
    kind: BlockKind,

    /// unique identifier
    path: Path,

    /// block depth starting from 0
    depth: usize,
}

pub enum BlockKind {
    Normal,
    ArrayLike,
}
