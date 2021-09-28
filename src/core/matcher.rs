use crate::core::{
    constraint::{Constraint, Predicate},
    language::Queryable,
    literal::match_string_pattern,
    node::ConsecutiveNodes,
    query::{
        MetavariableId, Query, SHISHO_NODE_ELLIPSIS, SHISHO_NODE_ELLIPSIS_METAVARIABLE,
        SHISHO_NODE_METAVARIABLE, SHISHO_NODE_METAVARIABLE_NAME,
    },
    tree::TreeView,
};
use anyhow::{anyhow, Result};
use itertools::Itertools;
use std::{collections::HashMap, convert::TryFrom, marker::PhantomData};

use super::{
    node::{Node, RootNode},
    tree::TreeTreverser,
};

pub struct QueryMatcher<'tree, 'query, T: Queryable> {
    traverser: TreeTreverser<'tree>,
    query: Box<RootNode<'query>>,

    items: Vec<MatchedItem<'tree>>,
    _marker: PhantomData<T>,
}

pub type UnverifiedMetavariable<'tree> = (MetavariableId, CaptureItem<'tree>);

#[derive(Debug, Default, Clone)]
pub struct MatcherState<'tree> {
    subtree: Option<ConsecutiveNodes<'tree>>,
    captures: Vec<UnverifiedMetavariable<'tree>>,
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

fn fold_capture<'tree>(capture_items: Vec<CaptureItem<'tree>>) -> Option<CaptureItem<'tree>> {
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

impl<'tree, 'query, T: Queryable> QueryMatcher<'tree, 'query, T> {
    pub fn new(view: &'tree TreeView<'tree, T>, query: &'query Query<T>) -> Self {
        QueryMatcher {
            query: query.root_node(),
            traverser: view.traverse(),
            items: vec![],

            _marker: PhantomData,
        }
    }

    fn match_sibillings(
        &self,
        tsibilings: Vec<&'tree Box<Node<'tree>>>,
        qsibilings: Vec<&'query Box<Node<'query>>>,
    ) -> Vec<(MatcherState<'tree>, Option<&Box<Node<'tree>>>)> {
        let mut queue: Vec<(usize, usize, Vec<UnverifiedMetavariable>)> = vec![(0, 0, vec![])];
        let mut result: Vec<(MatcherState, Option<&Box<Node<'tree>>>)> = vec![];

        while let Some((tidx, qidx, captures)) = queue.pop() {
            match (tsibilings.get(tidx), qsibilings.get(qidx)) {
                (t, None) => {
                    let nodes = tsibilings[..tidx.min(tsibilings.len())].to_vec();
                    result.push((
                        MatcherState {
                            subtree: ConsecutiveNodes::try_from(nodes).ok(),
                            captures,
                        },
                        t.map(|t| t.clone()),
                    ))
                }
                (Some(_tchild), Some(qchild)) if qchild.kind() == SHISHO_NODE_ELLIPSIS => {
                    let mut captured_nodes = vec![];
                    for tcidx in tidx..=tsibilings.len() {
                        queue.push((tcidx, qidx + 1, captures.clone()));
                        if let Some(tchild) = tsibilings.get(tcidx) {
                            captured_nodes.push(tchild.clone());
                        }
                    }
                }

                (Some(_tchild), Some(qchild))
                    if qchild.kind() == SHISHO_NODE_ELLIPSIS_METAVARIABLE =>
                {
                    let mid = MetavariableId(self.variable_name_of(&qchild).to_string());
                    let mut captured_nodes = vec![];
                    for tcidx in tidx..(tsibilings.len() + 1) {
                        queue.push((
                            tcidx,
                            qidx + 1,
                            [
                                vec![(mid.clone(), CaptureItem::from(captured_nodes.clone()))],
                                captures.clone(),
                            ]
                            .concat(),
                        ));
                        if let Some(tchild) = tsibilings.get(tcidx) {
                            captured_nodes.push(tchild.clone());
                        }
                    }
                }

                (Some(tchild), Some(qchild)) => {
                    for submatch in self.match_subtree(Some(tchild), Some(qchild)) {
                        queue.push((
                            tidx + 1,
                            qidx + 1,
                            [captures.clone(), submatch.captures].concat(),
                        ));
                    }
                }
                _ => (),
            }
        }
        result
    }

    fn match_subtree(
        &self,
        tnode: Option<&'tree Box<Node<'tree>>>,
        qnode: Option<&'query Box<Node<'query>>>,
    ) -> Vec<MatcherState<'tree>> {
        match (tnode, qnode) {
            (None, None) => {
                vec![Default::default()]
            }
            (Some(tnode), Some(qnode)) => match qnode.kind() {
                s if s == SHISHO_NODE_METAVARIABLE => {
                    let mid = MetavariableId(self.variable_name_of(&qnode).to_string());
                    let item = CaptureItem::from(vec![tnode]);
                    vec![MatcherState {
                        subtree: ConsecutiveNodes::try_from(vec![tnode]).ok(),
                        captures: vec![(mid, item)],
                    }]
                }
                _ if qnode.children.len() == 0 || T::is_leaf_like(&qnode) => {
                    self.match_leaf(&tnode, &qnode)
                }
                _ => {
                    if tnode.kind() != qnode.kind() {
                        return vec![];
                    }

                    let tchildren = tnode
                        .children
                        .iter()
                        .filter(|n| !T::is_skippable(n))
                        .collect();
                    let qchildren = qnode
                        .children
                        .iter()
                        .filter(|n| !T::is_skippable(n))
                        .collect();
                    self.match_sibillings(tchildren, qchildren)
                        .into_iter()
                        .filter_map(|(submatch, trailling)| {
                            if trailling.is_none() {
                                Some(MatcherState {
                                    subtree: ConsecutiveNodes::try_from(vec![tnode]).ok(),
                                    captures: submatch.captures,
                                })
                            } else {
                                None
                            }
                        })
                        .collect()
                }
            },
            _ => vec![],
        }
    }

    fn match_leaf(
        &self,
        tnode: &'tree Box<Node<'tree>>,
        qnode: &'query Box<Node<'query>>,
    ) -> Vec<MatcherState<'tree>> {
        if T::is_string_literal(tnode) && T::is_string_literal(qnode) {
            match_string_pattern(tnode.utf8_text(), qnode.utf8_text())
                .into_iter()
                .map(|captures| MatcherState {
                    subtree: ConsecutiveNodes::try_from(vec![tnode]).ok(),
                    captures,
                })
                .collect()
        } else {
            if tnode.kind() != qnode.kind() {
                return vec![];
            }

            let (tvalue, qvalue) = if tnode.is_named() {
                (tnode.utf8_text().to_string(), qnode.utf8_text().to_string())
            } else {
                (
                    T::normalize_annonymous_leaf(tnode.utf8_text()),
                    T::normalize_annonymous_leaf(qnode.utf8_text()),
                )
            };
            if tvalue == qvalue {
                vec![MatcherState {
                    subtree: ConsecutiveNodes::try_from(vec![tnode]).ok(),
                    captures: vec![],
                }]
            } else {
                vec![]
            }
        }
    }

    fn variable_name_of(&self, qnode: &Box<Node<'query>>) -> &'query str {
        qnode
            .children
            .iter()
            .find(|child| child.kind() == SHISHO_NODE_METAVARIABLE_NAME)
            .map(|child| child.utf8_text())
            .expect(
                format!(
                    "{} did not have {}",
                    SHISHO_NODE_ELLIPSIS_METAVARIABLE, SHISHO_NODE_METAVARIABLE_NAME
                )
                .as_str(),
            )
    }
}

impl<'tree, 'query, T> Iterator for QueryMatcher<'tree, 'query, T>
where
    T: Queryable,
{
    type Item = MatchedItem<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        let qnodes: Vec<&Box<Node<'query>>> = T::unwrap_root(&self.query)
            .into_iter()
            .filter(|n| !T::is_skippable(n))
            .collect();

        loop {
            if let Some(mitem) = self.items.pop() {
                return Some(mitem);
            }

            if let Some((depth, tnode)) = self.traverser.next() {
                let tnodes: Vec<&Box<Node>> = tnode
                    .children
                    .iter()
                    .filter(|n| !T::is_skippable(n))
                    .collect();
                let tcandidates =
                    (0..tnodes.len())
                        .map(|i| tnodes[i..].to_vec())
                        .chain(if depth == 0 {
                            vec![vec![tnode]].into_iter()
                        } else {
                            vec![].into_iter()
                        });
                for tsibilings in tcandidates {
                    let items = self
                        .match_sibillings(tsibilings, qnodes.clone())
                        .into_iter()
                        .filter_map(|(mitem, _)| Option::<MatchedItem>::from(mitem))
                        .collect::<Vec<MatchedItem>>();
                    self.items.extend(items);
                }
            } else {
                return None;
            }
        }
    }
}

#[derive(Debug)]
pub struct MatchedItem<'tree> {
    pub area: ConsecutiveNodes<'tree>,
    pub captures: HashMap<MetavariableId, CaptureItem<'tree>>,
}

impl<'tree> MatchedItem<'tree> {
    pub fn capture_of(&self, id: &MetavariableId) -> Option<&CaptureItem> {
        self.captures.get(&id)
    }

    pub fn satisfies_all<T: Queryable>(&self, constraints: &Vec<Constraint<T>>) -> Result<bool> {
        for c in constraints {
            if !self.satisfies(c)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn satisfies<T: Queryable>(&self, constraint: &Constraint<T>) -> Result<bool> {
        if !self.captures.contains_key(&constraint.target) {
            return Ok(false);
        }

        match &constraint.predicate {
            Predicate::MatchQuery(q) => {
                let captured_item = self.capture_of(&constraint.target).unwrap();
                match captured_item {
                    CaptureItem::Empty => Ok(false),
                    CaptureItem::Literal(_) => Err(anyhow!(
                        "match-query predicate for string literals is not supported"
                    )),
                    CaptureItem::Nodes(n) => Ok(n.as_vec().into_iter().any(|node| {
                        let ptree = TreeView::<T>::from((*node).clone());
                        let matches = ptree.matches(q).collect::<Vec<MatchedItem>>();
                        matches.len() > 0
                    })),
                }
            }
            Predicate::NotMatchQuery(q) => {
                let captured_item = self.capture_of(&constraint.target).unwrap();
                match captured_item {
                    CaptureItem::Empty => Ok(true),
                    CaptureItem::Literal(_) => Err(anyhow!(
                        "match-query predicate for string literals is not supported"
                    )),
                    CaptureItem::Nodes(n) => Ok(n.as_vec().into_iter().all(|node| {
                        let ptree = TreeView::<T>::from((*node).clone());
                        let matches = ptree.matches(q).collect::<Vec<MatchedItem>>();
                        matches.len() == 0
                    })),
                }
            }

            Predicate::MatchRegex(r) => {
                Ok(r.is_match(self.capture_of(&constraint.target).unwrap().as_str()))
            }
            Predicate::NotMatchRegex(r) => {
                Ok(!r.is_match(self.capture_of(&constraint.target).unwrap().as_str()))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureItem<'tree> {
    Empty,
    Literal(String),
    Nodes(ConsecutiveNodes<'tree>),
}

impl<'tree> CaptureItem<'tree> {
    pub fn as_str(&'tree self) -> &'tree str {
        match self {
            CaptureItem::Empty => "",
            CaptureItem::Literal(s) => s.as_str(),
            CaptureItem::Nodes(n) => n.utf8_text().unwrap(),
        }
    }
}

impl<'tree> From<Vec<&'tree Box<Node<'tree>>>> for CaptureItem<'tree> {
    fn from(value: Vec<&'tree Box<Node<'tree>>>) -> Self {
        if value.len() == 0 {
            Self::Empty
        } else {
            Self::Nodes(ConsecutiveNodes::try_from(value).unwrap())
        }
    }
}
