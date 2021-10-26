use std::{
    borrow::Cow,
    cell::{Ref, RefCell},
    rc::Rc,
};

use crate::core::{
    node::{Node, NodeLike, NodeType},
    source::NormalizedSource,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RewritableNode {
    kind: NodeType,

    start_byte: usize,
    end_byte: usize,

    start_position: tree_sitter::Point,
    end_position: tree_sitter::Point,

    pub source: Rc<RefCell<NormalizedSource>>,
    pub children: Vec<RewritableNode>,
}

impl NodeLike for RewritableNode {
    fn kind(&self) -> NodeType {
        self.kind.clone()
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
        std::borrow::Cow::Owned(
            source
                .as_str_between(self.start_byte(), self.end_byte())
                .unwrap()
                .to_string(),
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
        let source = self.source.borrow();
        callback(&source)
    }
}

impl<'tree> RewritableNode {
    pub fn from_node(n: &Node<'tree>, source: Rc<RefCell<NormalizedSource>>) -> Self {
        RewritableNode {
            kind: n.kind(),
            start_byte: n.start_byte(),
            end_byte: n.end_byte(),
            start_position: n.start_position(),
            end_position: n.end_position(),
            source: source.clone(),
            children: n
                .children
                .iter()
                .map(|x| Self::from_node(x, source.clone()))
                .collect(),
        }
    }
}
