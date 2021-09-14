use crate::core::{
    constraint::{Constraint, Predicate},
    language::Queryable,
    literal::match_string_pattern,
    node::ConsecutiveNodes,
    query::{
        MetavariableId, Query, SHISHO_NODE_ELLIPSIS, SHISHO_NODE_ELLIPSIS_METAVARIABLE,
        SHISHO_NODE_METAVARIABLE, SHISHO_NODE_METAVARIABLE_NAME,
    },
    tree::PartialTree,
};
use anyhow::{anyhow, Result};
use itertools::Itertools;
use std::collections::HashMap;

pub struct QueryMatcher<'tree, 'query, T>
where
    T: Queryable,
{
    cursor: Option<tree_sitter::TreeCursor<'tree>>,
    items: Vec<MatchedItem<'tree>>,

    tree: &'tree PartialTree<'tree, T>,
    query: &'query Query<T>,
}

pub type UnverifiedMetavariable<'tree> = (MetavariableId, CaptureItem<'tree>);

#[derive(Debug, Default, Clone)]
pub struct MatcherState<'tree> {
    pub(crate) subtree: Option<ConsecutiveNodes<'tree>>,
    pub(crate) captures: Vec<UnverifiedMetavariable<'tree>>,
}

impl<'tree, 'query, T> QueryMatcher<'tree, 'query, T>
where
    T: Queryable,
{
    fn yield_next_sibilings(&mut self) -> Option<Vec<tree_sitter::Node<'tree>>> {
        if let Some(cursor) = self.cursor.as_mut() {
            // collect sibilings
            let nodes = if let Some(parent) = cursor.node().parent() {
                parent.children(&mut parent.walk()).collect()
            } else {
                vec![cursor.node()]
            };

            // move to next leftmost child
            let got_root = 'o: loop {
                if cursor.goto_first_child() {
                    break 'o false;
                }
                while !cursor.goto_next_sibling() {
                    if !cursor.goto_parent() {
                        break 'o true;
                    }
                }
            };
            if got_root {
                self.cursor = None;
            }
            Some(nodes)
        } else {
            None
        }
    }
}

impl<'tree, 'query, T> QueryMatcher<'tree, 'query, T>
where
    T: Queryable,
{
    pub fn new(tree: &'tree PartialTree<'tree, T>, query: &'query Query<T>) -> Self {
        QueryMatcher {
            query,
            tree,
            cursor: Some(tree.root.walk()),
            items: vec![],
        }
    }

    fn match_sibillings(
        &self,
        tsibilings: Vec<tree_sitter::Node<'tree>>,
        qsibilings: Vec<tree_sitter::Node<'query>>,
    ) -> Vec<(MatcherState<'tree>, Option<tree_sitter::Node<'tree>>)> {
        let mut queue: Vec<(usize, usize, Vec<UnverifiedMetavariable>)> = vec![(0, 0, vec![])];
        let mut result: Vec<(MatcherState, Option<tree_sitter::Node<'tree>>)> = vec![];

        while let Some((tidx, qidx, captures)) = queue.pop() {
            match (tsibilings.get(tidx), qsibilings.get(qidx)) {
                (t, None) => {
                    let nodes = tsibilings[..tidx.min(tsibilings.len())].to_vec();
                    result.push((
                        MatcherState {
                            subtree: if nodes.len() > 0 {
                                Some(ConsecutiveNodes::from(nodes))
                            } else {
                                None
                            },
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
                    for submatch in self.match_subtree(Some(tchild.clone()), Some(qchild.clone())) {
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
        tnode: Option<tree_sitter::Node<'tree>>,
        qnode: Option<tree_sitter::Node<'query>>,
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
                        subtree: Some(ConsecutiveNodes::from(vec![tnode])),
                        captures: vec![(mid, item)],
                    }]
                }
                _ if qnode.child_count() == 0 || T::is_leaf_like(&qnode) => {
                    self.match_leaf(&tnode, &qnode)
                }
                _ => {
                    if tnode.kind() != qnode.kind() {
                        return vec![];
                    }

                    let tchildren = tnode
                        .children(&mut tnode.walk())
                        .filter(|n| !T::is_skippable(n))
                        .collect();
                    let qchildren = qnode
                        .children(&mut qnode.walk())
                        .filter(|n| !T::is_skippable(n))
                        .collect();
                    self.match_sibillings(tchildren, qchildren)
                        .into_iter()
                        .filter_map(|(submatch, trailling)| {
                            if trailling.is_none() {
                                Some(MatcherState {
                                    subtree: Some(ConsecutiveNodes::from(vec![tnode])),
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
        tnode: &tree_sitter::Node<'tree>,
        qnode: &tree_sitter::Node<'query>,
    ) -> Vec<MatcherState<'tree>> {
        if T::is_string_literal(tnode) && T::is_string_literal(qnode) {
            match_string_pattern(self.tree.value_of(tnode), self.query.value_of(qnode))
                .into_iter()
                .map(|captures| MatcherState {
                    subtree: Some(ConsecutiveNodes::from(vec![tnode.clone()])),
                    captures,
                })
                .collect()
        } else {
            if tnode.kind() != qnode.kind() {
                return vec![];
            }

            let (tvalue, qvalue) = if tnode.is_named() {
                (
                    self.tree.value_of(tnode).to_string(),
                    self.query.value_of(qnode).to_string(),
                )
            } else {
                (
                    T::normalize_annonymous_leaf(self.tree.value_of(tnode)),
                    T::normalize_annonymous_leaf(self.query.value_of(qnode)),
                )
            };
            if tvalue == qvalue {
                vec![MatcherState {
                    subtree: Some(ConsecutiveNodes::from(vec![tnode.clone()])),
                    captures: vec![],
                }]
            } else {
                vec![]
            }
        }
    }

    fn to_verified_capture(
        &self,
        capture_items: Vec<CaptureItem<'tree>>,
    ) -> Option<CaptureItem<'tree>> {
        let mut it = capture_items.into_iter();
        let first = it.next();
        it.fold(first, |acc, capture| match acc {
            Some(acc) => {
                if self.is_equivalent_capture(&acc, &capture) {
                    Some(capture)
                } else {
                    None
                }
            }
            None => None,
        })
    }

    fn is_equivalent_capture(&self, a: &CaptureItem<'tree>, b: &CaptureItem<'tree>) -> bool {
        a.to_string_with(self.tree.as_ref()) == b.to_string_with(self.tree.as_ref())
    }

    fn variable_name_of(&self, qnode: &tree_sitter::Node) -> &str {
        qnode
            .named_children(&mut qnode.walk())
            .find(|child| child.kind() == SHISHO_NODE_METAVARIABLE_NAME)
            .map(|child| self.query.value_of(&child))
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
        let qnodes = self.query.tsnodes();
        loop {
            if let Some(mitem) = self.items.pop() {
                return Some(mitem);
            }

            if let Some(tnodes) = self.yield_next_sibilings() {
                let tnodes: Vec<tree_sitter::Node> =
                    tnodes.into_iter().filter(|x| !T::is_skippable(x)).collect();
                for i in 0..tnodes.len() {
                    let tsibilings = tnodes[i..].to_vec();

                    'mitem: for (mitem, _trailling) in
                        self.match_sibillings(tsibilings, qnodes.clone())
                    {
                        let mut captures = HashMap::<MetavariableId, CaptureItem>::new();
                        for (mid, capture_items) in mitem
                            .captures
                            .into_iter()
                            .group_by(|k| k.0.clone())
                            .into_iter()
                        {
                            if mid == MetavariableId("_".into()) {
                                continue;
                            }
                            let capture_items = capture_items.into_iter().map(|x| x.1).collect();
                            if let Some(c) = self.to_verified_capture(capture_items) {
                                captures.insert(mid, c);
                            } else {
                                continue 'mitem;
                            }
                        }

                        self.items.push(MatchedItem {
                            raw: self.tree.as_ref(),
                            area: mitem.subtree.unwrap(),
                            captures,
                        });
                    }
                }
            } else {
                return None;
            }
        }
    }
}

#[derive(Debug)]
pub struct MatchedItem<'tree> {
    pub raw: &'tree [u8],
    pub area: ConsecutiveNodes<'tree>,
    pub captures: HashMap<MetavariableId, CaptureItem<'tree>>,
}

impl<'tree> MatchedItem<'tree> {
    pub fn value_of(&'tree self, id: &MetavariableId) -> Option<&'tree str> {
        let capture = self.captures.get(&id)?;

        match capture {
            CaptureItem::Empty => None,
            CaptureItem::Literal(s) => Some(s.as_str()),
            CaptureItem::Nodes(n) => Some(n.utf8_text(self.raw).unwrap()),
        }
    }

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
                        let ptree = PartialTree::<T>::new(node.clone(), self.raw);
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
                        let ptree = PartialTree::<T>::new(node.clone(), self.raw);
                        let matches = ptree.matches(q).collect::<Vec<MatchedItem>>();
                        matches.len() == 0
                    })),
                }
            }

            Predicate::MatchRegex(r) => Ok(r.is_match(self.value_of(&constraint.target).unwrap())),
            Predicate::NotMatchRegex(r) => {
                Ok(!r.is_match(self.value_of(&constraint.target).unwrap()))
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
    pub fn to_string_with(&'tree self, source: &'tree [u8]) -> &'tree str {
        match self {
            CaptureItem::Empty => "",
            CaptureItem::Literal(s) => s.as_str(),
            CaptureItem::Nodes(n) => n.utf8_text(source).unwrap(),
        }
    }
}

impl<'tree> From<Vec<tree_sitter::Node<'tree>>> for CaptureItem<'tree> {
    fn from(value: Vec<tree_sitter::Node<'tree>>) -> Self {
        if value.len() == 0 {
            Self::Empty
        } else {
            // TODO (y0n3uchy): check all capture items are consecutive
            Self::Nodes(ConsecutiveNodes::from(value))
        }
    }
}
