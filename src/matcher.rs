use anyhow::{anyhow, Result};
use tree_sitter::Point;

use crate::{
    constraint::{Constraint, Predicate},
    language::Queryable,
    query::{CaptureId, MetavariableId, Query, TOP_CAPTURE_ID_PREFIX},
    tree::PartialTree,
};
use std::collections::HashMap;

pub struct QueryMatcher<'tree, 'node, 'query, T>
where
    T: Queryable,
    'tree: 'query,
    'tree: 'node,
{
    cursor: tree_sitter::QueryCursor,
    query: &'query Query<T>,
    ptree: &'tree PartialTree<'tree, 'node, T>,
}

impl<'tree, 'node, 'query, T> QueryMatcher<'tree, 'node, 'query, T>
where
    T: Queryable,
    'tree: 'query,
{
    pub fn new(ptree: &'tree PartialTree<'tree, 'node, T>, query: &'query Query<T>) -> Self {
        let cursor = tree_sitter::QueryCursor::new();
        QueryMatcher {
            ptree,
            cursor,
            query,
        }
    }

    pub fn collect<'item>(mut self) -> Vec<MatchedItem<'item>>
    where
        'tree: 'item,
    {
        let raw = self.ptree.raw;
        let tsquery: &'query tree_sitter::Query = self.query.ts_query();

        let cidx_cid_map: &'query [String] = tsquery.capture_names();
        let cid_mvid_map = self.query.get_cid_mvid_map();

        self.cursor
            .matches(tsquery, self.ptree.as_ref().clone(), move |x| {
                x.utf8_text(raw).unwrap()
            })
            .map(|m| MatchedItem::try_from(m, raw, cidx_cid_map, &cid_mvid_map))
            .collect::<Vec<MatchedItem>>()
    }
}

#[derive(Debug)]
pub struct MatchedItem<'tree> {
    pub raw: &'tree [u8],
    pub top: ConsecutiveCaptureItems<'tree>,
    pub captures: HashMap<MetavariableId, CaptureItem<'tree>>,
}

impl<'tree> MatchedItem<'tree> {
    pub fn get_captured_string(&self, id: &MetavariableId) -> Option<&'tree str> {
        let capture = self.captures.get(&id)?;
        let t = capture.node.utf8_text(self.raw).unwrap();
        Some(t)
    }

    pub fn get_captured_node(&self, id: &MetavariableId) -> Option<tree_sitter::Node<'tree>> {
        let capture = self.captures.get(&id)?;
        Some(capture.node)
    }

    pub fn satisfies_all<T: Queryable>(&self, constraints: &Vec<Constraint<T>>) -> bool {
        constraints
            .iter()
            .all(|constraint| self.satisfies(constraint))
    }

    pub fn satisfies<T: Queryable>(&self, constraint: &Constraint<T>) -> bool {
        if !self.captures.contains_key(&constraint.target) {
            return false;
        }

        match &constraint.predicate {
            Predicate::MatchExactQuery(q) => {
                let item = self.get_captured_node(&constraint.target).unwrap();
                let ptree = PartialTree::<T>::new(item, self.raw);

                let matches = ptree.matches(q).collect();
                matches.len() == 1 && matches[0].top.as_vec().first().unwrap().node == item
            }
            Predicate::NotMatchExactQuery(q) => {
                let item = self.get_captured_node(&constraint.target).unwrap();
                let ptree = PartialTree::new(item, self.raw);

                let matches = ptree.matches(q).collect();
                matches.len() != 1 || matches[0].top.as_vec().first().unwrap().node != item
            }
            Predicate::MatchPartialQuery(q) => {
                let item = self.get_captured_node(&constraint.target).unwrap();
                let ptree = PartialTree::<T>::new(item, self.raw);

                ptree.matches(q).collect().len() > 0
            }
            Predicate::NotMatchPartialQuery(q) => {
                let item = self.get_captured_node(&constraint.target).unwrap();
                let ptree = PartialTree::new(item, self.raw);

                ptree.matches(q).collect().len() == 0
            }
            Predicate::MatchRegex(r) => {
                r.is_match(self.get_captured_string(&constraint.target).unwrap())
            }
            Predicate::NotMatchRegex(r) => {
                !r.is_match(self.get_captured_string(&constraint.target).unwrap())
            }
        }
    }
}

#[derive(Debug)]
pub struct ConsecutiveCaptureItems<'tree>(Vec<CaptureItem<'tree>>);

impl<'tree> ConsecutiveCaptureItems<'tree> {
    pub fn try_from(v: Vec<CaptureItem<'tree>>) -> Result<Self> {
        if v.len() == 0 {
            return Err(anyhow!("no item included"));
        }

        // TODO (y0n3uchy): check all capture items are consecutive

        Ok(Self(v))
    }

    pub fn as_vec(&self) -> &Vec<CaptureItem<'tree>> {
        &self.0
    }

    pub fn start_position(&self) -> Point {
        self.as_vec().first().unwrap().node.start_position()
    }

    pub fn end_position(&self) -> Point {
        self.as_vec().last().unwrap().node.end_position()
    }

    pub fn start_byte(&self) -> usize {
        self.as_vec().first().unwrap().node.start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.as_vec().last().unwrap().node.end_byte()
    }

    pub fn utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, core::str::Utf8Error> {
        core::str::from_utf8(&source[self.start_byte()..self.end_byte()])
    }
}

#[derive(Debug)]
pub struct CaptureItem<'tree> {
    pub node: tree_sitter::Node<'tree>,
}

impl<'tree> MatchedItem<'tree> {
    fn try_from<'query>(
        x: tree_sitter::QueryMatch<'tree>,
        raw: &'tree [u8],
        cidx_cid_map: &'query [String],
        cid_mvid_map: &'query HashMap<CaptureId, MetavariableId>,
    ) -> MatchedItem<'tree> {
        let mut captures = HashMap::<MetavariableId, CaptureItem>::new();
        let mut top = vec![];

        for y in x.captures {
            let capture_id = cidx_cid_map
                .get(y.index as usize)
                .and_then(|capture_id| Some(capture_id.clone()));
            match capture_id {
                Some(s) if s.as_str().starts_with(TOP_CAPTURE_ID_PREFIX) => {
                    // TODO (y0n3uchy): introduce appropriate abstraction layer to isolate `matcher` and `pattern` a little bit
                    top.push((s, CaptureItem { node: y.node }));
                }
                Some(s) => {
                    if let Some(metavariable_id) = cid_mvid_map.get(&CaptureId(s)) {
                        captures.insert(metavariable_id.clone(), CaptureItem { node: y.node });
                    }
                }
                None => (),
            }
        }

        top.sort_by(|x, y| x.0.cmp(&y.0));

        let top = top.into_iter().map(|item| item.1).collect();
        let top = ConsecutiveCaptureItems::try_from(top)
            .expect("internal error: global matching is invalid");

        MatchedItem { raw, top, captures }
    }
}
