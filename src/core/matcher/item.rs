use anyhow::Result;
use std::{collections::HashMap, convert::TryFrom};

use crate::core::{
    constraint::{Constraint, Predicate},
    language::Queryable,
    node::{ConsecutiveNodes, Node},
    query::MetavariableId,
    tree::RefTreeView,
};

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
            CaptureItem::Nodes(n) => n.as_str().unwrap(),
        }
    }
}

impl<'tree> From<Vec<&'tree Node<'tree>>> for CaptureItem<'tree> {
    fn from(value: Vec<&'tree Node<'tree>>) -> Self {
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Nodes(ConsecutiveNodes::try_from(value).unwrap())
        }
    }
}

pub type CaptureMap<'tree> = HashMap<MetavariableId, CaptureItem<'tree>>;

#[derive(Debug)]
pub struct MatchedItem<'tree> {
    pub area: ConsecutiveNodes<'tree>,
    pub captures: CaptureMap<'tree>,
}

impl<'tree> MatchedItem<'tree> {
    pub fn capture_of(&self, id: &MetavariableId) -> Option<&CaptureItem<'tree>> {
        self.captures.get(id)
    }

    pub fn satisfies_all<'c, T: Queryable + 'tree>(
        &self,
        constraints: &'c [Constraint<T>],
    ) -> Result<(bool, CaptureMap<'tree>)> {
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
    ) -> Result<(bool, CaptureMap<'tree>)> {
        let captured_item = self.capture_of(&constraint.target);
        if captured_item.is_none() {
            return Err(anyhow::anyhow!(
                "uncaptured variable was specified as constraint target: {}",
                constraint.target.0
            ));
        }
        let captured_item: &CaptureItem<'tree> = captured_item.unwrap();

        match &constraint.predicate {
            Predicate::MatchQuery(q) => match captured_item {
                CaptureItem::Empty => Ok((false, CaptureMap::new())),
                CaptureItem::Literal(_) => Err(anyhow::anyhow!(
                    "match-query predicate for string literals is not supported"
                )),
                CaptureItem::Nodes(n) => {
                    let matches = n
                        .as_vec()
                        .iter()
                        .map(|node: &&'tree Node<'tree>| {
                            let ptree = RefTreeView::<'tree, T>::from(*node);
                            let matches = ptree
                                .matches(&q.into())
                                .collect::<Result<Vec<MatchedItem<'tree>>>>()?;
                            let is_empty = matches.is_empty();
                            let captures = matches
                                .into_iter()
                                .map(|m: MatchedItem<'tree>| m.captures)
                                .fold(
                                    CaptureMap::new(),
                                    |mut acc: CaptureMap<'tree>, v: CaptureMap<'tree>| {
                                        acc.extend(v);
                                        acc
                                    },
                                );
                            Ok((!is_empty, captures))
                        })
                        .collect::<Result<Vec<(bool, CaptureMap<'tree>)>>>()?;
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
            },
            Predicate::NotMatchQuery(q) => match captured_item {
                CaptureItem::Empty => Ok((true, CaptureMap::new())),
                CaptureItem::Literal(_) => Err(anyhow::anyhow!(
                    "match-query predicate for string literals is not supported"
                )),
                CaptureItem::Nodes(n) => {
                    let mached = n
                        .as_vec()
                        .iter()
                        .map(|node| {
                            let ptree = RefTreeView::<T>::from(*node);
                            Ok(ptree
                                .matches(&q.into())
                                .collect::<Result<Vec<MatchedItem>>>()?
                                .is_empty())
                        })
                        .collect::<Result<Vec<bool>>>()?
                        .into_iter()
                        .all(|x| x);
                    Ok((mached, CaptureMap::new()))
                }
            },

            Predicate::MatchRegex(r) => Ok((r.is_match(captured_item.as_str()), CaptureMap::new())),
            Predicate::NotMatchRegex(r) => {
                Ok((!r.is_match(captured_item.as_str()), CaptureMap::new()))
            }
        }
    }
}
