use crate::core::node::{Node, NodeLike, NodeType, RootNode};

pub struct RewritableNode<'tree> {
    kind: NodeType,

    start_byte: usize,
    end_byte: usize,

    start_position: tree_sitter::Point,
    end_position: tree_sitter::Point,

    with_extra_newline: bool,

    pub(crate) source: &'tree [u8],
    pub children: Vec<RewritableNode<'tree>>,
}

impl<'tree> NodeLike<'tree> for RewritableNode<'tree> {
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

    fn as_str(&self) -> &'tree str {
        let last = if self.with_extra_newline {
            self.end_byte() - 1
        } else {
            self.end_byte()
        };
        core::str::from_utf8(&self.source[self.start_byte()..last]).unwrap()
    }
}

impl<'tree> From<RootNode<'tree>> for RewritableNode<'tree> {
    fn from(n: RootNode<'tree>) -> Self {
        let node: Node = n.into();
        node.into()
    }
}

impl<'tree> From<Node<'tree>> for RewritableNode<'tree> {
    fn from(n: Node<'tree>) -> Self {
        RewritableNode {
            kind: n.kind(),
            start_byte: n.start_byte(),
            end_byte: n.end_byte(),
            start_position: n.start_position(),
            end_position: n.end_position(),
            with_extra_newline: n.with_extra_newline,
            source: n.source,
            children: n.children.into_iter().map(|x| x.into()).collect(),
        }
    }
}
