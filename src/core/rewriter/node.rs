use std::{borrow::Cow, cell::RefCell, marker::PhantomData, rc::Rc};

use crate::core::{
    node::{Node, NodeArena, NodeLike, NodeLikeArena, NodeLikeId, NodeLikeRefWithId, NodeType},
    source::NormalizedSource,
    view::NodeLikeView,
};

pub type MutNodeId<'tree> = NodeLikeId<'tree, MutNode<'tree>>;
pub type MutNodeArena<'tree> = NodeLikeArena<'tree, MutNode<'tree>>;

#[derive(Debug, Clone, PartialEq)]
pub struct MutNode<'tree> {
    kind: NodeType,

    start_byte: usize,
    end_byte: usize,

    start_position: tree_sitter::Point,
    end_position: tree_sitter::Point,

    children: Vec<MutNodeId<'tree>>,
    pub source: Rc<RefCell<NormalizedSource>>,

    _marker: PhantomData<&'tree ()>,
}

impl<'tree> NodeLike<'tree> for MutNode<'tree> {
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

    fn children<V: NodeLikeView<'tree, Self>>(&'tree self, tview: &'tree V) -> Vec<&'tree Self> {
        self.children
            .iter()
            .map(|x| tview.get(*x).unwrap())
            .collect()
    }

    fn indexed_children<V: NodeLikeView<'tree, Self>>(
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
        let source = self.source.borrow();
        callback(&source)
    }
}

impl<'tree> MutNode<'tree> {
    pub fn from_node<'ntree, 'b>(
        n: &'ntree Node<'ntree>,
        base_arena: &'ntree NodeArena<'ntree>,

        source: Rc<RefCell<NormalizedSource>>,
        new_arena: &'tree mut MutNodeArena<'tree>,
    ) -> MutNodeId<'tree> {
        let mut children = vec![];
        for x in n.children {
            children.push(Self::from_node(
                base_arena.get(x).unwrap(),
                base_arena,
                source,
                new_arena,
            ));
        }

        new_arena.alloc(MutNode {
            kind: n.kind(),
            start_byte: n.start_byte(),
            end_byte: n.end_byte(),
            start_position: n.start_position(),
            end_position: n.end_position(),
            source,
            children,
            _marker: PhantomData,
        })
    }
}
