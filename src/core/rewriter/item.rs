use crate::core::{
    language::Queryable,
    matcher::{CaptureItem, CaptureMap},
    node::NodeLike,
    query::MetavariableId,
    rewriter::node::MutNode,
    source::NormalizedSource,
    tree::RootedTreeLike,
};
use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use super::node::{MutNodeArena, MutNodeId};

pub type MetavariableMap<'tree, T: Queryable> = HashMap<MetavariableId, MutCaptureItem<'tree, T>>;

pub fn from_capture_map<
    'tree,
    'btree,
    T: Queryable,
    N: NodeLike<'btree>,
    V: RootedTreeLike<'btree, N>,
>(
    bview: &'btree V,
    cmap: &'btree CaptureMap<'btree, N>,
) -> MetavariableMap<'tree, T> {
    let mut newmap = MetavariableMap::<'_, T>::new();
    for (k, v) in cmap {
        newmap.insert(k.clone(), from_capture_item(bview, v));
    }
    newmap
}

pub fn from_capture_item<
    'tree,
    'btree,
    T: Queryable,
    N: NodeLike<'btree>,
    V: RootedTreeLike<'btree, N>,
>(
    bview: &'btree V,
    cmap: &'btree CaptureItem<'btree, N>,
) -> MutCaptureItem<'tree, T> {
    match cmap {
        CaptureItem::Empty => MutCaptureItem::Empty,
        CaptureItem::Literal(l) => MutCaptureItem::Literal(l.clone()),
        CaptureItem::Nodes(nodes) => {
            let mut arena = MutNodeArena::new();

            let source = nodes.to_string();
            let source = Rc::new(RefCell::new(source.into()));

            let byte_offset = nodes.start_byte();
            let position_offset = nodes.start_position();
            let ids = nodes
                .as_vec()
                .into_iter()
                .map(|n| {
                    MutNode::from_node(
                        n.node,
                        bview,
                        source.clone(),
                        &mut arena,
                        byte_offset,
                        position_offset,
                    )
                })
                .collect::<Vec<MutNodeId>>();

            let root_id = if ids.len() > 1 {
                MutNode::create_unifier(ids, &mut arena)
            } else {
                ids[0]
            };
            MutCaptureItem::Tree {
                root_id,
                source,
                arena,
                _marker: PhantomData,
            }
        }
    }
}

#[derive(Debug)]
pub enum MutCaptureItem<'tree, T: Queryable> {
    Empty,
    Literal(String),
    Tree {
        root_id: MutNodeId<'tree>,
        source: Rc<RefCell<NormalizedSource>>,
        arena: MutNodeArena<'tree>,
        _marker: PhantomData<T>,
    },
}

impl<'tree, T: Queryable> ToString for MutCaptureItem<'tree, T> {
    fn to_string(&self) -> String {
        match &self {
            MutCaptureItem::Empty => "".to_string(),
            MutCaptureItem::Literal(s) => s.to_string(),
            MutCaptureItem::Tree { root_id, arena, .. } => {
                arena.get(root_id.clone()).unwrap().as_cow().to_string()
            }
        }
    }
}
