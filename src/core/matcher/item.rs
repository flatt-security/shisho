use anyhow::Result;
use std::collections::HashMap;
use std::convert::TryFrom;

use crate::core::node::{NodeLikeId, NodeLikeRefWithId, Range, TSPoint};
use crate::core::pattern::Pattern;
use crate::core::query::Query;
use crate::core::ruleset::constraint::{Constraint, ConstraintPredicate};
use crate::core::source::NormalizedSource;
use crate::core::tree::TreeView;
use crate::core::{language::Queryable, node::NodeLike, query::MetavariableId};

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureItem<'tree, N: NodeLike<'tree>> {
    Empty,
    Literal(String),
    Nodes(ConsecutiveNodes<'tree, N>),
}

impl<'tree, 'item, N: NodeLike<'tree>> CaptureItem<'tree, N> {
    pub fn to_string(&self) -> String {
        match self {
            CaptureItem::Empty => "".to_string(),
            CaptureItem::Literal(s) => s.to_string(),
            CaptureItem::Nodes(n) => n.to_string(),
        }
    }
}

impl<'tree, N: NodeLike<'tree>> From<Vec<NodeLikeRefWithId<'tree, N>>> for CaptureItem<'tree, N> {
    fn from(value: Vec<NodeLikeRefWithId<'tree, N>>) -> Self {
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Nodes(ConsecutiveNodes::try_from(value).unwrap())
        }
    }
}

pub type CaptureMap<'tree, N> = HashMap<MetavariableId, CaptureItem<'tree, N>>;

#[derive(Debug, Clone)]
pub struct MatchedItem<'tree, N: NodeLike<'tree>> {
    pub area: ConsecutiveNodes<'tree, N>,
    pub captures: CaptureMap<'tree, N>,
}

impl<'tree, N: NodeLike<'tree>> MatchedItem<'tree, N> {
    pub fn capture_of(&self, id: &MetavariableId) -> Option<&CaptureItem<'tree, N>> {
        self.captures.get(id)
    }

    pub fn into_unconstrained(self) -> UnconstrainedMatchedItem<'tree, N> {
        UnconstrainedMatchedItem {
            area: self.area,
            captures: self.captures,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnconstrainedMatchedItem<'tree, N: NodeLike<'tree>> {
    pub area: ConsecutiveNodes<'tree, N>,
    pub captures: CaptureMap<'tree, N>,
}

impl<'tree, N: NodeLike<'tree>> UnconstrainedMatchedItem<'tree, N> {
    pub fn capture_of(&self, id: &MetavariableId) -> Option<&CaptureItem<'tree, N>> {
        self.captures.get(id)
    }

    pub fn into_constrained(self) -> MatchedItem<'tree, N> {
        MatchedItem {
            area: self.area,
            captures: self.captures,
        }
    }

    pub fn apply_constraints<'c, T: Queryable + 'tree>(
        self,
        view: &'tree TreeView<'tree, T, N>,
        constraints: &'c [Constraint<T>],
    ) -> Result<Vec<MatchedItem<'tree, N>>> {
        let mut r = vec![self];
        for c in constraints {
            r = r
                .into_iter()
                .map(|x| x.apply_constraint(view, c))
                .collect::<Result<Vec<Vec<MatchedItem<'tree, N>>>>>()?
                .into_iter()
                .flatten()
                .map(|x| x.into_unconstrained())
                .collect();
        }
        Ok(r.into_iter().map(|x| x.into_constrained()).collect())
    }

    fn apply_constraint<'c, T: Queryable + 'tree>(
        self,
        view: &'tree TreeView<'tree, T, N>,
        constraint: &'c Constraint<T>,
    ) -> Result<Vec<MatchedItem<'tree, N>>> {
        let captured_item = self.captures.get(&constraint.target);
        if captured_item.is_none() {
            return Err(anyhow::anyhow!(
                "uncaptured variable was specified as constraint target: {}",
                constraint.target.0
            ));
        }
        let captured_item = captured_item.unwrap().clone();

        match &constraint.predicate {
            ConstraintPredicate::MatchQuery(q) => {
                self.apply_constraint_with_item(view, q, &captured_item)
            }
            ConstraintPredicate::NotMatchQuery(q) => Ok(
                if self
                    .apply_constraint_with_item(view, q, &captured_item)?
                    .is_empty()
                {
                    vec![self.into_constrained()]
                } else {
                    vec![]
                },
            ),
            ConstraintPredicate::MatchAnyOfQuery(qs) => Ok(qs
                .into_iter()
                .map(|q| self.apply_constraint_with_item(view, q, &captured_item))
                .collect::<Result<Vec<Vec<MatchedItem<_>>>>>()?
                .into_iter()
                .flatten()
                .collect()),
            ConstraintPredicate::NotMatchAnyOfQuery(qs) => {
                let matches = qs
                    .into_iter()
                    .map(|q| self.apply_constraint_with_item(view, q, &captured_item))
                    .collect::<Result<Vec<Vec<_>>>>()?;
                Ok(if matches.into_iter().all(|x| x.is_empty()) {
                    vec![self.into_constrained()]
                } else {
                    vec![]
                })
            }
            ConstraintPredicate::MatchRegex(r) => Ok(if r.is_match(&captured_item.to_string()) {
                vec![self.into_constrained()]
            } else {
                vec![]
            }),
            ConstraintPredicate::NotMatchRegex(r) => {
                Ok(if !r.is_match(&captured_item.to_string()) {
                    vec![self.into_constrained()]
                } else {
                    vec![]
                })
            }
            ConstraintPredicate::MatchAnyOfRegex(rs) => Ok(
                if rs
                    .into_iter()
                    .any(|r| r.is_match(&captured_item.to_string()))
                {
                    vec![self.into_constrained()]
                } else {
                    vec![]
                },
            ),
            ConstraintPredicate::NotMatchAnyOfRegex(rs) => Ok(
                if !rs
                    .into_iter()
                    .any(|r| r.is_match(&captured_item.to_string()))
                {
                    vec![self.into_constrained()]
                } else {
                    vec![]
                },
            ),
            ConstraintPredicate::BeAnyOf(candidates) => Ok(
                if candidates
                    .into_iter()
                    .any(|r| r.as_str() == captured_item.to_string())
                {
                    vec![self.into_constrained()]
                } else {
                    vec![]
                },
            ),
            ConstraintPredicate::NotBeAnyOf(candidates) => Ok(
                if !candidates
                    .into_iter()
                    .any(|r| r.as_str() == captured_item.to_string())
                {
                    vec![self.into_constrained()]
                } else {
                    vec![]
                },
            ),
        }
    }

    fn apply_constraint_with_item<T: Queryable + 'tree>(
        &self,
        view: &'tree TreeView<'tree, T, N>,

        pattern: &Pattern<T>,
        item: &CaptureItem<'tree, N>,
    ) -> Result<Vec<MatchedItem<'tree, N>>> {
        match item {
            CaptureItem::Empty => Ok(vec![self.clone().into_constrained()]),
            CaptureItem::Literal(_) => Err(anyhow::anyhow!(
                "match-query predicate for string literals is not supported"
            )),
            CaptureItem::Nodes(n) => Ok(view
                .matches_under_sibilings(
                    n.as_vec().iter().map(|r| r.id).collect(),
                    &Query {
                        pattern: pattern.into(),
                        constraints: &vec![],
                    },
                )
                .collect::<Result<Vec<MatchedItem<'tree, N>>>>()?
                .into_iter()
                .map(|mitem| {
                    let mut x = self.clone().into_constrained();
                    x.captures.extend(mitem.captures);
                    x
                })
                .collect()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsecutiveNodes<'tree, N: NodeLike<'tree>> {
    inner: Vec<NodeLikeRefWithId<'tree, N>>,
}

impl<'tree, N: NodeLike<'tree>> TryFrom<Vec<(NodeLikeId<'tree, N>, &'tree N)>>
    for ConsecutiveNodes<'tree, N>
{
    type Error = anyhow::Error;
    fn try_from(inner: Vec<(NodeLikeId<'tree, N>, &'tree N)>) -> Result<Self, Self::Error> {
        // TODO (y0n3uchy): check all capture items are consecutive
        if inner.is_empty() {
            Err(anyhow::anyhow!(
                "internal error; ConsecutiveNodes was generated from empty vec."
            ))
        } else {
            Ok(ConsecutiveNodes {
                inner: inner
                    .into_iter()
                    .map(|x| NodeLikeRefWithId { id: x.0, node: x.1 })
                    .collect(),
            })
        }
    }
}

impl<'tree, N: NodeLike<'tree>> TryFrom<Vec<NodeLikeRefWithId<'tree, N>>>
    for ConsecutiveNodes<'tree, N>
{
    type Error = anyhow::Error;
    fn try_from(inner: Vec<NodeLikeRefWithId<'tree, N>>) -> Result<Self, Self::Error> {
        // TODO (y0n3uchy): check all capture items are consecutive
        if inner.is_empty() {
            Err(anyhow::anyhow!(
                "internal error; ConsecutiveNodes was generated from empty vec."
            ))
        } else {
            Ok(ConsecutiveNodes { inner })
        }
    }
}

impl<'tree, N: NodeLike<'tree>> TryFrom<Vec<ConsecutiveNodes<'tree, N>>>
    for ConsecutiveNodes<'tree, N>
{
    type Error = anyhow::Error;

    fn try_from(cns: Vec<ConsecutiveNodes<'tree, N>>) -> Result<Self, Self::Error> {
        // TODO (y0n3uchy): check all capture items are consecutive
        if cns.is_empty() {
            Err(anyhow::anyhow!(
                "internal error; ConsecutiveNodes was generated from empty vec."
            ))
        } else {
            Ok(ConsecutiveNodes {
                inner: cns.into_iter().map(|cn| cn.inner).flatten().collect(),
            })
        }
    }
}

impl<'tree, N: NodeLike<'tree>> ConsecutiveNodes<'tree, N> {
    pub fn as_vec(&self) -> &Vec<NodeLikeRefWithId<'tree, N>> {
        &self.inner
    }

    pub fn push(&mut self, n: NodeLikeRefWithId<'tree, N>) {
        self.inner.push(n)
    }

    pub fn range<T: Queryable>(&self) -> Range {
        Range {
            start: T::range(self.as_vec().first().unwrap().node).start,
            end: T::range(self.as_vec().last().unwrap().node).end,
        }
    }

    pub fn len(&self) -> usize {
        self.as_vec().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn start_position(&self) -> TSPoint {
        self.as_vec().first().unwrap().node.start_position()
    }

    pub fn end_position(&self) -> TSPoint {
        self.as_vec().last().unwrap().node.end_position()
    }

    pub fn start_byte(&self) -> usize {
        self.as_vec().first().unwrap().node.start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.as_vec().last().unwrap().node.end_byte()
    }

    #[inline]
    pub fn to_string(&self) -> String {
        self.as_vec()
            .first()
            .unwrap()
            .node
            .with_source(|source: &NormalizedSource| {
                source
                    .as_str_between(self.start_byte(), self.end_byte())
                    .unwrap()
                    .to_string()
            })
    }
}
