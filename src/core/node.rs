use crate::core::language::Queryable;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq)]
pub struct ConsecutiveNodes<'tree>(Vec<tree_sitter::Node<'tree>>);

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

impl<'tree> From<Vec<tree_sitter::Node<'tree>>> for ConsecutiveNodes<'tree> {
    fn from(value: Vec<tree_sitter::Node<'tree>>) -> Self {
        if value.len() == 0 {
            panic!("internal error; ConsecutiveNodes was generated from empty vec.");
        }
        ConsecutiveNodes(value)
    }
}

impl<'tree> From<Vec<ConsecutiveNodes<'tree>>> for ConsecutiveNodes<'tree> {
    fn from(cns: Vec<ConsecutiveNodes<'tree>>) -> Self {
        ConsecutiveNodes(cns.into_iter().map(|cn| cn.0).flatten().collect())
    }
}

impl<'tree> ConsecutiveNodes<'tree> {
    pub fn as_vec(&self) -> &Vec<tree_sitter::Node<'tree>> {
        &self.0
    }

    pub fn push(&mut self, n: tree_sitter::Node<'tree>) {
        self.0.push(n)
    }

    pub fn start_position(&self) -> tree_sitter::Point {
        self.as_vec().first().unwrap().start_position()
    }

    pub fn end_position(&self) -> tree_sitter::Point {
        self.as_vec().last().unwrap().end_position()
    }

    pub fn range<T: Queryable + 'static>(&self, source: &[u8]) -> Range {
        Range {
            start: T::range(self.as_vec().first().unwrap(), source).start,
            end: T::range(self.as_vec().last().unwrap(), source).end,
        }
    }

    pub fn start_byte(&self) -> usize {
        self.as_vec().first().unwrap().start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.as_vec().last().unwrap().end_byte()
    }

    pub fn utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, core::str::Utf8Error> {
        core::str::from_utf8(&source[self.start_byte()..self.end_byte()])
    }
}
