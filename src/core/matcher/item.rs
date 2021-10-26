use anyhow::Result;
use std::collections::HashMap;
use std::convert::TryFrom;

use crate::core::node::Range;
use crate::core::source::NormalizedSource;
use crate::core::{
    constraint::{Constraint, Predicate},
    language::Queryable,
    node::NodeLike,
    pattern::PatternWithConstraints,
    query::MetavariableId,
    tree::RefTreeView,
};

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureItem<'tree, N: NodeLike> {
    Empty,
    Literal(String),
    Nodes(ConsecutiveNodes<'tree, N>),
}

impl<'tree, N: NodeLike> CaptureItem<'tree, N> {
    pub fn matches<T: Queryable + 'tree>(
        &self,
        q: &PatternWithConstraints<T>,
    ) -> Result<(bool, CaptureMap<'tree, N>)> {
        match self {
            CaptureItem::Empty => Ok((false, CaptureMap::new())),
            CaptureItem::Literal(_) => Err(anyhow::anyhow!(
                "match-query predicate for string literals is not supported"
            )),
            CaptureItem::Nodes(n) => {
                let matches = n
                    .as_vec()
                    .iter()
                    .map(|node: &&'tree N| {
                        let ptree = RefTreeView::<'tree, T, N>::from(*node);
                        let matches = ptree
                            .matches(&q.into())
                            .collect::<Result<Vec<MatchedItem<'tree, N>>>>()?;
                        let is_empty = matches.is_empty();
                        let captures = matches
                            .into_iter()
                            .map(|m: MatchedItem<'tree, N>| m.captures)
                            .fold(
                                CaptureMap::new(),
                                |mut acc: CaptureMap<'tree, N>, v: CaptureMap<'tree, N>| {
                                    acc.extend(v);
                                    acc
                                },
                            );
                        Ok((!is_empty, captures))
                    })
                    .collect::<Result<Vec<(bool, CaptureMap<'tree, N>)>>>()?;
                let matched_at_least_one = matches.iter().any(|m| m.0);
                let captures =
                    matches
                        .into_iter()
                        .map(|m| m.1)
                        .fold(CaptureMap::new(), |mut acc, v| {
                            acc.extend(v);
                            acc
                        });
                Ok((matched_at_least_one, captures))
            }
        }
    }

    pub fn to_string(&'tree self) -> String {
        match self {
            CaptureItem::Empty => "".to_string(),
            CaptureItem::Literal(s) => s.to_string(),
            CaptureItem::Nodes(n) => n.to_string(),
        }
    }
}

impl<'tree, N: NodeLike> From<Vec<&'tree N>> for CaptureItem<'tree, N> {
    fn from(value: Vec<&'tree N>) -> Self {
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Nodes(ConsecutiveNodes::try_from(value).unwrap())
        }
    }
}

pub type CaptureMap<'tree, N> = HashMap<MetavariableId, CaptureItem<'tree, N>>;

#[derive(Debug, Clone)]
pub struct MatchedItem<'tree, N: NodeLike> {
    pub area: ConsecutiveNodes<'tree, N>,
    pub captures: CaptureMap<'tree, N>,
}

impl<'tree, N: NodeLike> MatchedItem<'tree, N> {
    pub fn capture_of(&self, id: &MetavariableId) -> Option<&CaptureItem<'tree, N>> {
        self.captures.get(id)
    }

    pub fn satisfies_all<'c, T: Queryable + 'tree>(
        &self,
        constraints: &'c [Constraint<T>],
    ) -> Result<(bool, CaptureMap<'tree, N>)> {
        let mut items = CaptureMap::new();
        for c in constraints {
            let (satisfied, mitems) = self.satisfies(c)?;
            if satisfied {
                items.extend(mitems);
            } else {
                return Ok((false, CaptureMap::new()));
            }
        }
        Ok((true, items))
    }

    pub fn satisfies<'c, T: Queryable + 'tree>(
        &self,
        constraint: &'c Constraint<T>,
    ) -> Result<(bool, CaptureMap<'tree, N>)> {
        let captured_item = self.capture_of(&constraint.target);
        if captured_item.is_none() {
            return Err(anyhow::anyhow!(
                "uncaptured variable was specified as constraint target: {}",
                constraint.target.0
            ));
        }
        let captured_item: &CaptureItem<'tree, N> = captured_item.unwrap();

        match &constraint.predicate {
            Predicate::MatchQuery(q) => captured_item.matches(q),
            Predicate::NotMatchQuery(q) => captured_item
                .matches(q)
                .map(|(matched, _)| (!matched, HashMap::new())),
            Predicate::MatchAnyOfQuery(qs) => {
                let matches = qs
                    .into_iter()
                    .map(|q| captured_item.matches(q))
                    .collect::<Result<Vec<(bool, CaptureMap<N>)>>>()?;
                let matched_at_least_one = matches.iter().any(|m| m.0);
                let captures =
                    matches
                        .into_iter()
                        .map(|m| m.1)
                        .fold(CaptureMap::new(), |mut acc, v| {
                            acc.extend(v);
                            acc
                        });
                Ok((matched_at_least_one, captures))
            }
            Predicate::NotMatchAnyOfQuery(qs) => {
                let matches = qs
                    .into_iter()
                    .map(|q| captured_item.matches(q))
                    .collect::<Result<Vec<(bool, CaptureMap<N>)>>>()?;
                Ok((!matches.iter().any(|m| m.0), CaptureMap::new()))
            }

            Predicate::MatchRegex(r) => {
                Ok((r.is_match(&captured_item.to_string()), CaptureMap::new()))
            }
            Predicate::NotMatchRegex(r) => {
                Ok((!r.is_match(&captured_item.to_string()), CaptureMap::new()))
            }
            Predicate::MatchAnyOfRegex(rs) => Ok((
                rs.into_iter()
                    .any(|r| r.is_match(&captured_item.to_string())),
                CaptureMap::new(),
            )),
            Predicate::NotMatchAnyOfRegex(rs) => Ok((
                !rs.into_iter()
                    .any(|r| r.is_match(&captured_item.to_string())),
                CaptureMap::new(),
            )),

            Predicate::BeAnyOf(candidates) => Ok((
                candidates
                    .into_iter()
                    .any(|r| r.as_str() == captured_item.to_string()),
                CaptureMap::new(),
            )),
            Predicate::NotBeAnyOf(candidates) => Ok((
                !candidates
                    .into_iter()
                    .any(|r| r.as_str() == captured_item.to_string()),
                CaptureMap::new(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsecutiveNodes<'tree, N: NodeLike> {
    inner: Vec<&'tree N>,
}

impl<'tree, N: NodeLike> TryFrom<Vec<&'tree N>> for ConsecutiveNodes<'tree, N> {
    type Error = anyhow::Error;
    fn try_from(inner: Vec<&'tree N>) -> Result<Self, Self::Error> {
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

impl<'tree, N: NodeLike> TryFrom<Vec<ConsecutiveNodes<'tree, N>>> for ConsecutiveNodes<'tree, N> {
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

impl<'tree, N: NodeLike> ConsecutiveNodes<'tree, N> {
    pub fn as_vec(&self) -> &Vec<&'tree N> {
        &self.inner
    }

    pub fn push(&mut self, n: &'tree N) {
        self.inner.push(n)
    }

    pub fn range<T: Queryable>(&self) -> Range {
        Range {
            start: T::range(*self.as_vec().first().unwrap()).start,
            end: T::range(*self.as_vec().last().unwrap()).end,
        }
    }

    pub fn len(&self) -> usize {
        self.as_vec().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn start_position(&self) -> tree_sitter::Point {
        self.as_vec().first().unwrap().start_position()
    }

    pub fn end_position(&self) -> tree_sitter::Point {
        self.as_vec().last().unwrap().end_position()
    }

    pub fn start_byte(&self) -> usize {
        self.as_vec().first().unwrap().start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.as_vec().last().unwrap().end_byte()
    }

    #[inline]
    pub fn to_string(&self) -> String {
        self.as_vec()
            .first()
            .unwrap()
            .with_source(|source: &NormalizedSource| {
                source
                    .as_str_between(self.start_byte(), self.end_byte())
                    .unwrap()
                    .to_string()
            })
    }
}
