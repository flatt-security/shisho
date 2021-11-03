use itertools::Itertools;

use crate::core::{matcher::CaptureItem, node::NodeLike, query::MetavariableId};

use super::ConsecutiveNodes;
use super::{CaptureMap, UnconstrainedMatchedItem};

pub type UnverifiedMetavariable<'tree, N> = (MetavariableId, CaptureItem<'tree, N>);

#[derive(Debug, Default, Clone)]
pub struct MatcherState<'tree, N: NodeLike<'tree>> {
    pub(crate) subtree: Option<ConsecutiveNodes<'tree, N>>,
    pub(crate) captures: Vec<UnverifiedMetavariable<'tree, N>>,
}

impl<'tree, N: NodeLike<'tree>> From<MatcherState<'tree, N>>
    for Option<UnconstrainedMatchedItem<'tree, N>>
{
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

        Some(UnconstrainedMatchedItem {
            area: value.subtree.unwrap(),
            captures,
        })
    }
}

fn fold_capture<'tree, N: NodeLike<'tree>>(
    capture_items: Vec<CaptureItem<'tree, N>>,
) -> Option<CaptureItem<'tree, N>> {
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
