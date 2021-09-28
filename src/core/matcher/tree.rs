use super::{
    super::{
        language::Queryable,
        node::ConsecutiveNodes,
        node::{Node, RootNode},
        query::{
            MetavariableId, Query, SHISHO_NODE_ELLIPSIS, SHISHO_NODE_ELLIPSIS_METAVARIABLE,
            SHISHO_NODE_METAVARIABLE, SHISHO_NODE_METAVARIABLE_NAME,
        },
        tree::{TreeTreverser, TreeView},
    },
    item::CaptureItem,
    item::MatchedItem,
    literal,
    state::{MatcherState, UnverifiedMetavariable},
};

use std::{convert::TryFrom, marker::PhantomData};

pub struct TreeMatcher<'tree, 'query, T: Queryable> {
    traverser: TreeTreverser<'tree>,
    query: Box<RootNode<'query>>,

    /// local state for implementing `Iterator`/
    items: Vec<MatchedItem<'tree>>,

    /// just a marker
    _marker: PhantomData<T>,
}

impl<'tree, 'query, T: Queryable> TreeMatcher<'tree, 'query, T> {
    pub fn new(view: &'tree TreeView<'tree, T>, query: &'query Query<T>) -> Self {
        TreeMatcher {
            query: query.root_node(),
            traverser: view.traverse(),
            items: vec![],

            _marker: PhantomData,
        }
    }
}

impl<'tree, 'query, T: Queryable> TreeMatcher<'tree, 'query, T> {
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
                        t.copied(),
                    ))
                }
                (Some(_tchild), Some(qchild)) if qchild.kind() == SHISHO_NODE_ELLIPSIS => {
                    let mut captured_nodes = vec![];
                    for tcidx in tidx..=tsibilings.len() {
                        queue.push((tcidx, qidx + 1, captures.clone()));
                        if let Some(tchild) = tsibilings.get(tcidx) {
                            captured_nodes.push(tchild);
                        }
                    }
                }

                (Some(_tchild), Some(qchild))
                    if qchild.kind() == SHISHO_NODE_ELLIPSIS_METAVARIABLE =>
                {
                    let mid = MetavariableId(self.variable_name_of(qchild).to_string());
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
                            captured_nodes.push(tchild);
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
                    let mid = MetavariableId(self.variable_name_of(qnode).to_string());
                    let item = CaptureItem::from(vec![tnode]);
                    vec![MatcherState {
                        subtree: ConsecutiveNodes::try_from(vec![tnode]).ok(),
                        captures: vec![(mid, item)],
                    }]
                }
                _ if qnode.children.is_empty() || T::is_leaf_like(qnode) => {
                    self.match_leaf(tnode, qnode)
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
            literal::match_string_pattern(tnode.utf8_text(), qnode.utf8_text())
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

impl<'tree, 'query, T> Iterator for TreeMatcher<'tree, 'query, T>
where
    T: Queryable,
{
    type Item = MatchedItem<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        let qnodes: Vec<&Box<Node<'query>>> = T::unwrap_root(&self.query)
            .iter()
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
