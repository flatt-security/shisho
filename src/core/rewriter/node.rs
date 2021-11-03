use std::{borrow::Cow, cell::RefCell, marker::PhantomData, rc::Rc};

use crate::core::{
    node::{NodeLike, NodeLikeArena, NodeLikeId, NodeLikeRefWithId, NodeType, TSPoint},
    source::NormalizedSource,
    tree::RootedTreeLike,
};

pub type MutNodeId<'tree> = NodeLikeId<'tree, MutNode<'tree>>;
pub type MutNodeArena<'tree> = NodeLikeArena<'tree, MutNode<'tree>>;

#[derive(Debug, Clone, PartialEq)]
pub struct MutNode<'tree> {
    kind: NodeType,

    start_byte: usize,
    end_byte: usize,

    start_position: TSPoint,
    end_position: TSPoint,

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

    fn start_position(&self) -> TSPoint {
        self.start_position
    }

    fn end_position(&self) -> TSPoint {
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
        let source = self.source.borrow();
        callback(&source)
    }
}

impl<'tree> MutNode<'tree> {
    pub fn from_node<'btree, N: NodeLike<'btree>, V: RootedTreeLike<'btree, N>>(
        base_nodelike: &'btree N,
        base_view: &'btree V,

        source: Rc<RefCell<NormalizedSource>>,
        new_arena: &mut MutNodeArena<'tree>,

        byte_offset: usize,
        position_offset: TSPoint,
    ) -> MutNodeId<'tree> {
        let mut children = vec![];
        for x in base_nodelike.children(base_view) {
            children.push(Self::from_node(
                x,
                base_view,
                source.clone(),
                new_arena,
                byte_offset,
                position_offset,
            ));
        }

        new_arena.alloc(MutNode {
            kind: base_nodelike.kind(),
            start_byte: base_nodelike.start_byte() - byte_offset,
            end_byte: base_nodelike.end_byte() - byte_offset,
            start_position: base_nodelike.start_position() - position_offset,
            end_position: base_nodelike.end_position() - position_offset,
            source,
            children,
            _marker: PhantomData,
        })
    }

    pub fn create_unifier<'btree>(
        targets: Vec<MutNodeId<'tree>>,
        arena: &mut MutNodeArena<'tree>,
    ) -> MutNodeId<'tree> {
        let start = arena.get(targets.first().unwrap().clone()).unwrap().clone();
        let end = arena.get(targets.last().unwrap().clone()).unwrap().clone();

        arena.alloc(MutNode {
            kind: NodeType::Unifier,
            start_byte: start.start_byte(),
            end_byte: end.end_byte(),
            start_position: start.start_position(),
            end_position: end.end_position(),
            source: start.source.clone(),
            children: targets.clone(),
            _marker: PhantomData,
        })
    }
}
