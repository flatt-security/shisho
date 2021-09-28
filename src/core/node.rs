use std::convert::TryFrom;

use crate::core::language::Queryable;
use serde::{Deserialize, Serialize};

use super::tree::TreeView;

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
    pub(crate) source: &'tree [u8],

    pub children: Vec<Box<Node<'tree>>>,
}

impl<'tree> Node<'tree> {
    pub fn kind(&self) -> &'tree str {
        self.inner.kind()
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

    pub fn utf8_text(&self) -> &'tree str {
        core::str::from_utf8(&self.source[self.start_byte()..self.end_byte()]).unwrap()
    }

    pub fn is_named(&self) -> bool {
        self.inner.is_named()
    }

    pub fn from_tsnode(tsnode: tree_sitter::Node<'tree>, source: &'tree [u8]) -> Box<Self> {
        let children: Vec<Box<Self>> = tsnode
            .children(&mut tsnode.walk())
            .map(|c| Self::from_tsnode(c, source))
            .collect();
        Box::new(Node {
            inner: tsnode,
            children,
            source,
        })
    }

    pub fn to_view<T: Queryable>(self: Box<Self>) -> TreeView<'tree, T> {
        TreeView::from(self)
    }
}

pub struct RootNode<'tree>(Node<'tree>);

impl<'tree> RootNode<'tree> {
    pub fn new(node: Node<'tree>) -> Self {
        Self(node)
    }

    pub fn as_node(&self) -> &Node<'tree> {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsecutiveNodes<'tree> {
    inner: Vec<&'tree Box<Node<'tree>>>,
    source: &'tree [u8],
}

impl<'tree> TryFrom<Vec<&'tree Box<Node<'tree>>>> for ConsecutiveNodes<'tree> {
    type Error = anyhow::Error;
    fn try_from(inner: Vec<&'tree Box<Node<'tree>>>) -> Result<Self, Self::Error> {
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
    pub fn as_vec(&self) -> &Vec<&'tree Box<Node<'tree>>> {
        &self.inner
    }

    pub fn push(&mut self, n: &'tree Box<Node<'tree>>) {
        self.inner.push(n)
    }

    pub fn start_position(&self) -> tree_sitter::Point {
        self.as_vec().first().unwrap().start_position()
    }

    pub fn end_position(&self) -> tree_sitter::Point {
        self.as_vec().last().unwrap().end_position()
    }

    pub fn range<T: Queryable + 'static>(&self) -> Range {
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

    pub fn utf8_text(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(&self.source[self.start_byte()..self.end_byte()])
    }
}
