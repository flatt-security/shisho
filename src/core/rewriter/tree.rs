use super::node::MutNode;
use crate::core::{matcher::CaptureMap, node::NodeLike, query::MetavariableId};
use std::collections::HashMap;

pub type MetavariableMap<'tree> = HashMap<MetavariableId, CapturedValue<'tree>>;

pub fn from_capture_map<'ntree, 'tree, N: NodeLike<'tree>>(
    _: CaptureMap<'tree, N>,
) -> MetavariableMap<'ntree> {
    todo!("convert all")
}

#[derive(Debug, Clone, PartialEq)]
pub enum CapturedValue<'tree> {
    Empty,
    Literal(String),
    Node(MutNode<'tree>),
}

impl<'tree> ToString for CapturedValue<'tree> {
    fn to_string(&self) -> String {
        match &self {
            CapturedValue::Empty => "".to_string(),
            CapturedValue::Literal(s) => s.to_string(),
            CapturedValue::Node(n) => n.as_cow().to_string(),
        }
    }
}
