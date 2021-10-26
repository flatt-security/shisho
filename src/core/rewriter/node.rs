use std::{borrow::Cow, cell::RefCell, rc::Rc};

use crate::core::node::{Node, NodeLike, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct RewritableNode {
    kind: NodeType,

    start_byte: usize,
    end_byte: usize,

    start_position: tree_sitter::Point,
    end_position: tree_sitter::Point,

    with_extra_newline: bool,

    pub source: Rc<RefCell<Vec<u8>>>,
    pub children: Vec<RewritableNode>,
}

impl NodeLike for RewritableNode {
    fn kind(&self) -> NodeType {
        self.kind.clone()
    }

    fn with_extra_newline(&self) -> bool {
        self.with_extra_newline
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
        let source = self.source.borrow();
        let last = if self.with_extra_newline {
            self.end_byte() - 1
        } else {
            self.end_byte()
        };

        std::borrow::Cow::Owned(
            core::str::from_utf8(&source[self.start_byte()..last])
                .unwrap()
                .to_string(),
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
        let source = self.source.borrow();
        callback(source.as_slice())
    }
}

impl<'tree> RewritableNode {
    pub fn from_node(n: &Node<'tree>, source: Rc<RefCell<Vec<u8>>>) -> Self {
        RewritableNode {
            kind: n.kind(),
            start_byte: n.start_byte(),
            end_byte: n.end_byte(),
            start_position: n.start_position(),
            end_position: n.end_position(),
            with_extra_newline: n.with_extra_newline(),
            source: source.clone(),
            children: n
                .children
                .iter()
                .map(|x| Self::from_node(x, source.clone()))
                .collect(),
        }
    }
}
