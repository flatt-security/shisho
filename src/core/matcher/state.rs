use itertools::Itertools;

use crate::core::{
    matcher::{CaptureItem, MatchedItem},
    node::NodeLike,
    query::MetavariableId,
};

use super::CaptureMap;
use super::ConsecutiveNodes;

pub type UnverifiedMetavariable<'tree, N> = (MetavariableId, CaptureItem<'tree, N>);

#[derive(Debug, Default, Clone)]
pub struct MatcherState<'tree, N: NodeLike> {
    pub(crate) subtree: Option<ConsecutiveNodes<'tree, N>>,
    pub(crate) captures: Vec<UnverifiedMetavariable<'tree, N>>,
}

impl<'tree, N: NodeLike> From<MatcherState<'tree, N>> for Option<MatchedItem<'tree, N>> {
    fn from(value: MatcherState<'tree, N>) -> Self {
        let mut captures = CaptureMap::<N>::new();

        let captures_per_mid = value.captures.into_iter().group_by(|k| k.0.clone());
        for (mid, citems) in captures_per_mid.into_iter() {
            if mid == MetavariableId("_".into()) {
                continue;
            }
            let capture_items = citems.into_iter().map(|x| x.1).collect();
            if let Some(c) = fold_capture(capture_items) {
                captures.insert(mid, c);
            } else {
                return None;
            }
        }

        Some(MatchedItem {
            area: value.subtree.unwrap(),
            captures,
        })
    }
}

fn fold_capture<N: NodeLike>(capture_items: Vec<CaptureItem<'_, N>>) -> Option<CaptureItem<'_, N>> {
    let mut it = capture_items.into_iter();
    let first = it.next();
    it.fold(first, |acc, capture| match acc {
        Some(acc) => {
            if acc.to_string() == capture.to_string() {
                Some(capture)
            } else {
                None
            }
        }
        None => None,
    })
}
