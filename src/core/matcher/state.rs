use itertools::Itertools;

use crate::core::{
    matcher::{CaptureItem, MatchedItem},
    node::ConsecutiveNodes,
    query::MetavariableId,
};
use std::collections::HashMap;

pub type UnverifiedMetavariable<'tree> = (MetavariableId, CaptureItem<'tree>);

#[derive(Debug, Default, Clone)]
pub struct MatcherState<'tree> {
    pub(crate) subtree: Option<ConsecutiveNodes<'tree>>,
    pub(crate) captures: Vec<UnverifiedMetavariable<'tree>>,
}

impl<'tree> From<MatcherState<'tree>> for Option<MatchedItem<'tree>> {
    fn from(value: MatcherState<'tree>) -> Self {
        let mut captures = HashMap::<MetavariableId, CaptureItem>::new();

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

fn fold_capture(capture_items: Vec<CaptureItem<'_>>) -> Option<CaptureItem<'_>> {
    let mut it = capture_items.into_iter();
    let first = it.next();
    it.fold(first, |acc, capture| match acc {
        Some(acc) => {
            if acc.as_str() == capture.as_str() {
                Some(capture)
            } else {
                None
            }
        }
        None => None,
    })
}
