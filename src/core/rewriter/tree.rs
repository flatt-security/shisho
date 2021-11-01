use super::node::MutNode;
use crate::core::{matcher::CaptureMap, node::NodeLike, query::MetavariableId};
use std::collections::HashMap;

pub type MetavariableMap = HashMap<MetavariableId, CapturedValue>;

pub fn from_capture_map<'tree, N: NodeLike>(_: CaptureMap<'tree, N>) -> MetavariableMap {
    todo!("convert all")
}

#[derive(Debug, Clone, PartialEq)]
pub enum CapturedValue {
    Empty,
    Literal(String),
    Node(MutNode),
}

impl ToString for CapturedValue {
    fn to_string(&self) -> String {
        match &self {
            CapturedValue::Empty => "".to_string(),
            CapturedValue::Literal(s) => s.to_string(),
            CapturedValue::Node(n) => n.as_cow().to_string(),
        }
    }
}
