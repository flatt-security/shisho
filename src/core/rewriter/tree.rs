use super::node::RewritableNode;
use crate::core::{
    language::Queryable, matcher::CaptureMap, node::NodeLike, query::MetavariableId,
    source::NormalizedSource,
};
use std::{collections::HashMap, marker::PhantomData};

pub struct NormalizedRewritableTree<'tree, T> {
    pub root: RewritableNode,
    pub source: &'tree NormalizedSource,
    _marker: PhantomData<T>,
}

impl<'tree, T> NormalizedRewritableTree<'tree, T>
where
    T: Queryable,
{
    pub fn new(view_root: RewritableNode, source: &'tree NormalizedSource) -> Self {
        Self {
            root: view_root,
            source,
            _marker: PhantomData,
        }
    }
}

pub type RewritableNodeArena = ();

pub type MetavariableMap = HashMap<MetavariableId, CapturedValue>;

pub fn from_capture_map<'tree, N: NodeLike>(_: CaptureMap<'tree, N>) -> MetavariableMap {
    todo!("convert all")
}

#[derive(Debug, Clone)]
pub struct CapturedValue {
    arena: RewritableNodeArena,
    pub kind: CapturedValueKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CapturedValueKind {
    Empty,
    Literal(String),
    Node(RewritableNode),
}

impl ToString for CapturedValue {
    fn to_string(&self) -> String {
        match &self.kind {
            CapturedValueKind::Empty => "".to_string(),
            CapturedValueKind::Literal(s) => s.to_string(),
            CapturedValueKind::Node(n) => n.as_cow().to_string(),
        }
    }
}
