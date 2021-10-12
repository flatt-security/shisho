use crate::core::{
    language::Queryable,
    matcher::{
        match_string_pattern, CaptureItem, MatchedItem, MatcherState, UnverifiedMetavariable,
    },
    node::ConsecutiveNodes,
    node::{Node, NodeType, RootNode},
    query::QueryPattern,
    tree::TreeTreverser,
};

use std::{convert::TryFrom, marker::PhantomData};

/// `TreeMatcher` iterates possible matches between query and tree to traverse.
pub struct TreeMatcher<'tree, 'query, T: Queryable> {
    /// Tree to traverse
    traverser: TreeTreverser<'tree>,

    /// Query
    query: &'query RootNode<'query>,

    /// local state for implementing `Iterator`/    
    items: Vec<MatchedItem<'tree>>,

    /// just a marker
    _marker: PhantomData<T>,
}

impl<'tree, 'query, T: Queryable> TreeMatcher<'tree, 'query, T> {
    pub fn new(traverser: TreeTreverser<'tree>, query: &'query QueryPattern<T>) -> Self {
        TreeMatcher {
            query: &query.root_node,
            traverser,
            items: vec![],

            _marker: PhantomData,
        }
    }
}

impl<'tree, 'query, T: Queryable> TreeMatcher<'tree, 'query, T> {
    /// `match_sibilings` takes two sibiling nodes (one for the code tree to search, one for the query tree) and returns zero or more matches.    
    /// Each element of return value accompanies with a value with `Option<&Node>`.
    ///
    /// If the value is `None`, the element describes a full match of two sibiling nodes.
    /// - Example Input:
    ///     - tsibilings: A B C D E F
    ///     - qsibilings: A B C D E F
    /// - Example Output:
    ///     - (MatcherState {subtree: ABCDEF, .. }, None)
    ///
    /// Otherwise, the element describes a partial match of two sibiling nodes, and the value equals to the first unmatched element.
    /// - Example Input:
    ///     - tsibilings: A B C D E F
    ///     - qsibilings: A B C D
    /// - Example Output:
    ///     - (MatcherState {subtree: ABCDE, .. }, Some(E))
    ///
    /// When `qsibilings` includes ellipis nodes, it may returns multile matches.
    /// - Example Input:
    ///     - tsibilings: A B C D E F
    ///     - qsibilings: A B [ellipsis]
    /// - Example Output:
    ///     - (MatcherState {subtree: AB, .. }, Some(C))
    ///     - (MatcherState {subtree: ABC, .. }, Some(D))
    ///     - (MatcherState {subtree: ABCD, .. }, Some(E))
    ///     - (MatcherState {subtree: ABCDE, .. }, Some(F))
    ///     - (MatcherState {subtree: ABCDEF, .. }, None)    
    fn match_sibilings(
        &self,
        tsibilings: Vec<&'tree Node<'tree>>,
        qsibilings: Vec<&'query Node<'query>>,
    ) -> Vec<(MatcherState<'tree>, Option<&Node<'tree>>)> {
        let mut queue: Vec<(usize, usize, Vec<UnverifiedMetavariable>)> = vec![(0, 0, vec![])];
        let mut result: Vec<(MatcherState, Option<&Node<'tree>>)> = vec![];

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

                (None, Some(qchild)) => match qchild.kind() {
                    NodeType::Ellipsis if qidx == qsibilings.len() - 1 => {
                        let nodes = tsibilings[..tidx.min(tsibilings.len())].to_vec();
                        result.push((
                            MatcherState {
                                subtree: ConsecutiveNodes::try_from(nodes).ok(),
                                captures,
                            },
                            None,
                        ))
                    }
                    NodeType::EllipsisMetavariable(mid) if qidx == qsibilings.len() - 1 => {
                        let nodes = tsibilings[..tidx.min(tsibilings.len())].to_vec();
                        result.push((
                            MatcherState {
                                subtree: ConsecutiveNodes::try_from(nodes).ok(),
                                captures: [
                                    captures.clone(),
                                    vec![(mid, CaptureItem::from(vec![]))],
                                ]
                                .concat(),
                            },
                            None,
                        ))
                    }
                    _ => {}
                },
                (Some(tchild), Some(qchild)) => match qchild.kind() {
                    NodeType::Ellipsis => {
                        // NOTE: this loop must end with `tsibilings.len()`
                        for tcidx in tidx..=tsibilings.len() {
                            queue.push((tcidx, qidx + 1, captures.clone()));
                        }
                    }
                    NodeType::EllipsisMetavariable(mid) => {
                        let mut captured_nodes = vec![];
                        // NOTE: this loop must end with `tsibilings.len()`
                        for tcidx in tidx..=tsibilings.len() {
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
                    _ => {
                        for submatch in self.match_intermediate_node(Some(tchild), Some(qchild)) {
                            queue.push((
                                tidx + 1,
                                qidx + 1,
                                [captures.clone(), submatch.captures].concat(),
                            ));
                        }
                    }
                },
            }
        }
        result
    }

    fn match_intermediate_node(
        &self,
        tnode: Option<&'tree Node<'tree>>,
        qnode: Option<&'query Node<'query>>,
    ) -> Vec<MatcherState<'tree>> {
        match (tnode, qnode) {
            (None, None) =>
            // treat as a match
            {
                vec![Default::default()]
            }
            (Some(tnode), Some(qnode)) =>
            // check the equality of two nodes and get possible matches
            {
                match qnode.kind() {
                    NodeType::Metavariable(mid) => {
                        // MATCH: a metavariable node matches any node.
                        let item = CaptureItem::from(vec![tnode]);
                        vec![MatcherState {
                            subtree: ConsecutiveNodes::try_from(vec![tnode]).ok(),
                            captures: vec![(mid, item)],
                        }]
                    }
                    NodeType::Ellipsis | NodeType::EllipsisMetavariable(_) => {
                        // ERROR: any ellipsis node should handled outside this function
                        panic!(
                            "internal error: ellipsis nodes were given to match_intermediate_node"
                        )
                    }
                    _ if qnode.children.is_empty() || T::is_leaf_like(qnode) => {
                        // MATCH or UNMATCH: leaf nodes should be compared more.
                        self.match_leaf(tnode, qnode)
                    }
                    _ => {
                        // MATCH or UNMATCH: intermediate nodes should be compared more.

                        // intermediate nodes match if and only if:
                        // (1) both nodes have a same kind
                        // (2) both nodes' children match

                        // (1): check kinds
                        if tnode.kind() != qnode.kind() {
                            return vec![];
                        }

                        // (2): get matches of children
                        self.match_sibilings(
                            tnode
                                .children
                                .iter()
                                .filter(|n| !T::is_skippable(n))
                                .collect(),
                            qnode
                                .children
                                .iter()
                                .filter(|n| !T::is_skippable(n))
                                .collect(),
                        )
                        .into_iter()
                        .filter_map(|(submatch, trailling)| {
                            if trailling.is_none() {
                                // in this case children match completely.
                                Some(MatcherState {
                                    subtree: ConsecutiveNodes::try_from(vec![tnode]).ok(),
                                    captures: submatch.captures,
                                })
                            } else {
                                // in this case children match partially.
                                None
                            }
                        })
                        .collect()
                    }
                }
            }
            _ =>
            // treat as no match
            {
                vec![]
            }
        }
    }

    /// `match_leaf` validates the equality of two leaf nodes with `NodeType::Normal`.
    fn match_leaf(
        &self,
        tnode: &'tree Node<'tree>,
        qnode: &'query Node<'query>,
    ) -> Vec<MatcherState<'tree>> {
        assert!(matches!(tnode.kind(), NodeType::Normal(_)));
        assert!(matches!(qnode.kind(), NodeType::Normal(_)));

        if T::is_string_literal(tnode) && T::is_string_literal(qnode) {
            // when both of tnode and qnode is string literal, use string matcher to check the equality of them
            match_string_pattern(tnode.as_str(), qnode.as_str())
                .into_iter()
                .map(|captures| MatcherState {
                    subtree: ConsecutiveNodes::try_from(vec![tnode]).ok(),
                    captures,
                })
                .collect()
        } else {
            // otherwise, two nodes match if and only if:
            // (1) two nodes are same kind
            // (2) two nodes are same as string
            if tnode.kind() == qnode.kind() && T::node_value_eq(tnode, qnode) {
                vec![MatcherState {
                    subtree: ConsecutiveNodes::try_from(vec![tnode]).ok(),
                    captures: vec![],
                }]
            } else {
                vec![]
            }
        }
    }
}

impl<'tree, 'query, T> Iterator for TreeMatcher<'tree, 'query, T>
where
    T: Queryable,
{
    type Item = MatchedItem<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        let qnodes: Vec<&Node<'query>> = T::unwrap_root(self.query)
            .iter()
            .filter(|n| !T::is_skippable(n))
            .collect();

        loop {
            if let Some(mitem) = self.items.pop() {
                return Some(mitem);
            }

            if let Some((depth, tnode)) = self.traverser.next() {
                let tnodes: Vec<&Node<'tree>> = tnode
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
                        .match_sibilings(tsibilings, qnodes.clone())
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
