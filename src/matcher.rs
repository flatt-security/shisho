use anyhow::{anyhow, Result};

use crate::{
    language::Queryable,
    query::{CaptureId, MetavariableId, Query, GLOBAL_CAPTURE_ID},
    tree::Tree,
};
use std::collections::HashMap;

pub struct QueryMatcher<'tree, 'query, T>
where
    T: Queryable,
    'tree: 'query,
{
    cursor: tree_sitter::QueryCursor,
    query: &'query Query<T>,
    tree: &'tree Tree<'tree, T>,
}

impl<'tree, 'query, T> QueryMatcher<'tree, 'query, T>
where
    T: Queryable,
    'tree: 'query,
{
    pub fn new(tree: &'tree Tree<'tree, T>, query: &'query Query<T>) -> Self {
        let cursor = tree_sitter::QueryCursor::new();
        QueryMatcher {
            tree,
            cursor,
            query,
        }
    }

    pub fn collect<'item>(mut self) -> Vec<MatchedItem<'item>>
    where
        'tree: 'item,
    {
        let raw = self.tree.raw;
        let tsquery: &'query tree_sitter::Query = self.query.ts_query();

        let cidx_cid_map: &'query [String] = tsquery.capture_names();
        let cid_mvid_map = self.query.get_cid_mvid_map();

        self.cursor
            .matches(tsquery, self.tree.ts_tree().root_node(), move |x| {
                x.utf8_text(raw).unwrap()
            })
            .map(|m| MatchedItem::try_from(m, cidx_cid_map, &cid_mvid_map))
            .collect::<Vec<MatchedItem>>()
    }
}

#[derive(Debug)]
pub struct MatchedItem<'tree> {
    pub global: CaptureItem<'tree>,
    pub captures: HashMap<MetavariableId, CaptureItem<'tree>>,
}

#[derive(Debug)]
pub struct CaptureItem<'tree> {
    pub node: tree_sitter::Node<'tree>,
}

impl<'tree> MatchedItem<'tree> {
    fn try_from<'query>(
        x: tree_sitter::QueryMatch<'tree>,
        cidx_cid_map: &'query [String],
        cid_mvid_map: &'query HashMap<CaptureId, MetavariableId>,
    ) -> MatchedItem<'tree> {
        let mut captures_map = HashMap::<MetavariableId, CaptureItem>::new();

        let mut global = None;
        for y in x.captures {
            let capture_id = cidx_cid_map
                .get(y.index as usize)
                .and_then(|capture_id| Some(capture_id.clone()));
            match capture_id {
                Some(s) if s.as_str() == GLOBAL_CAPTURE_ID => {
                    global = Some(CaptureItem { node: y.node });
                }
                Some(s) => {
                    if let Some(metavariable_id) = cid_mvid_map.get(&CaptureId(s)) {
                        captures_map.insert(metavariable_id.clone(), CaptureItem { node: y.node });
                    }
                }
                None => (),
            }
        }

        match global {
            Some(g) => MatchedItem {
                global: g,
                captures: captures_map,
            },
            None => panic!(
                "internal error: no capture for global matching found"
            ),
        }
    }
}
