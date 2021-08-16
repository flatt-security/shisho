use anyhow::{anyhow, Result};
use tree_sitter::Point;

use crate::{
    constraint::{Constraint, Predicate},
    language::Queryable,
    query::{CaptureId, MetavariableId, Query, TOP_CAPTURE_ID_PREFIX},
    tree::PartialTree,
};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};

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
        let query = self.query;

        self.cursor
            .matches(tsquery, self.ptree.as_ref().clone(), move |x| {
                x.utf8_text(raw).unwrap()
            })
            .map(|m| MatchedItem::from(m, raw, query))
            .collect::<Vec<MatchedItem>>()
    }
}

#[derive(Debug)]
pub struct MatchedItem<'tree> {
    pub raw: &'tree [u8],
    pub top: ConsecutiveCaptureItems<'tree>,
    pub captures: HashMap<MetavariableId, ConsecutiveCaptureItems<'tree>>,
}

impl<'tree> MatchedItem<'tree> {
    pub fn get_captured_string(&self, id: &MetavariableId) -> Option<&'tree str> {
        let capture = self.captures.get(&id)?;
        let t = capture.utf8_text(self.raw).unwrap();
        Some(t)
    }

    pub fn get_captured(&self, id: &MetavariableId) -> Option<&ConsecutiveCaptureItems<'tree>> {
        self.captures.get(&id)
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
                // let item = self.get_captured(&constraint.target).unwrap();
                // let ptree = PartialTree::<T>::new(item, self.raw);

                // let matches = ptree.matches(q).collect();
                // matches.len() == 1 && matches[0].top.as_vec().first().unwrap().node == item
                true
            }
            Predicate::NotMatchExactQuery(q) => {
                // let item = self.get_captured(&constraint.target).unwrap();
                // let ptree = PartialTree::new(item, self.raw);

                // let matches = ptree.matches(q).collect();
                // matches.len() != 1 || matches[0].top.as_vec().first().unwrap().node != item
                true
            }
            Predicate::MatchPartialQuery(q) => {
                // let item = self.get_captured(&constraint.target).unwrap();
                // let ptree = PartialTree::<T>::new(item, self.raw);

                // ptree.matches(q).collect().len() > 0
                true
            }
            Predicate::NotMatchPartialQuery(q) => {
                // let item = self.get_captured(&constraint.target).unwrap();
                // let ptree = PartialTree::new(item, self.raw);

                // ptree.matches(q).collect().len() == 0
                true
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
pub struct ConsecutiveCaptureItems<'tree>(Vec<tree_sitter::Node<'tree>>);

impl<'tree> TryFrom<Vec<tree_sitter::Node<'tree>>> for ConsecutiveCaptureItems<'tree> {
    type Error = anyhow::Error;

    fn try_from(value: Vec<tree_sitter::Node<'tree>>) -> Result<Self, Self::Error> {
        if value.len() == 0 {
            return Err(anyhow!("no item included"));
        }

        // TODO (y0n3uchy): check all capture items are consecutive

        Ok(Self(value))
    }
}

impl<'tree> ConsecutiveCaptureItems<'tree> {
    pub fn as_vec(&self) -> &Vec<tree_sitter::Node<'tree>> {
        &self.0
    }

    pub fn push(&mut self, n: tree_sitter::Node<'tree>) {
        self.0.push(n)
    }

    pub fn start_position(&self) -> Point {
        self.as_vec().first().unwrap().start_position()
    }

    pub fn end_position(&self) -> Point {
        self.as_vec().last().unwrap().end_position()
    }

    pub fn start_byte(&self) -> usize {
        self.as_vec().first().unwrap().start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.as_vec().last().unwrap().end_byte()
    }

    pub fn utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, core::str::Utf8Error> {
        core::str::from_utf8(&source[self.start_byte()..self.end_byte()])
    }
}

#[derive(Debug)]
pub struct CaptureItem<'tree> {
    pub node: Vec<tree_sitter::Node<'tree>>,
}

impl<'tree> MatchedItem<'tree> {
    fn from<'query, T: Queryable>(
        x: tree_sitter::QueryMatch<'tree>,
        raw: &'tree [u8],
        query: &'query Query<T>,
    ) -> MatchedItem<'tree> {
        // map from QueryCapture index to Capture Name
        let cidx_cid_map: &'query [String] = query.ts_query().capture_names();

        // map from capture id to metavariable id
        let cid_mvid_map = query.get_cid_mvid_map();

        // captures for each metavariable IDs
        let mut meta_captures = HashMap::<MetavariableId, ConsecutiveCaptureItems>::new();

        // captures for top-level nodes
        let mut top_captures = vec![];

        // part up all the captures into meta_captures or top_captures
        for capture in x.captures {
            let capture_id = cidx_cid_map
                .get(capture.index as usize)
                .and_then(|capture_id| Some(capture_id.clone()));
            match capture_id {
                Some(s) if s.as_str().starts_with(TOP_CAPTURE_ID_PREFIX) => {
                    // TODO (y0n3uchy): introduce appropriate abstraction layer to isolate `matcher` and `pattern` a little bit
                    top_captures.push((s, capture.node));
                }
                Some(s) => {
                    if let Some(metavariable_id) = cid_mvid_map.get(&CaptureId(s)) {
                        if let Some(v) = meta_captures.get_mut(metavariable_id) {
                            v.push(capture.node);
                        } else {
                            let v = vec![capture.node].try_into().unwrap();
                            meta_captures.insert(metavariable_id.clone(), v);
                        }
                    }
                }
                None => (),
            }
        }

        top_captures.sort_by(|x, y| x.0.cmp(&y.0));
        let top_captures: Vec<tree_sitter::Node> =
            top_captures.into_iter().map(|item| item.1).collect();
        let top_captures = ConsecutiveCaptureItems::try_from(top_captures)
            .expect("internal error: global matching is invalid");

        MatchedItem {
            raw,
            top: top_captures,
            captures: meta_captures,
        }
    }
}
