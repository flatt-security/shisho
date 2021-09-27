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
    source: &'tree [u8],

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

    pub fn utf8_text<'a>(&self) -> &'tree str {
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

    pub fn to_view<T: Queryable>(self: &'tree Box<Self>) -> TreeView<'tree, T> {
        TreeView::new(&self, self.source)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsecutiveNodes<'tree> {
    inner: Vec<&'tree Box<Node<'tree>>>,
    source: &'tree [u8],
}

impl<'tree> From<Vec<&'tree Box<Node<'tree>>>> for ConsecutiveNodes<'tree> {
    fn from(value: Vec<&'tree Box<Node<'tree>>>) -> Self {
        if value.len() == 0 {
            panic!("internal error; ConsecutiveNodes was generated from empty vec.");
        }
        let source = value.get(0).unwrap().source;
        ConsecutiveNodes {
            inner: value,
            source,
        }
    }
}

impl<'tree> From<Vec<ConsecutiveNodes<'tree>>> for ConsecutiveNodes<'tree> {
    fn from(cns: Vec<ConsecutiveNodes<'tree>>) -> Self {
        if cns.len() == 0 {
            panic!("internal error; ConsecutiveNodes was generated from empty vec.");
        }
        let source = cns.get(0).unwrap().source;
        ConsecutiveNodes {
            inner: cns.into_iter().map(|cn| cn.inner).flatten().collect(),
            source,
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

    pub fn range<T: Queryable + 'static>(&self, source: &[u8]) -> Range {
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
