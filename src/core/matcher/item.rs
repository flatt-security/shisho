use anyhow::Result;
use std::{collections::HashMap, convert::TryFrom};

use crate::core::{
    constraint::{Constraint, Predicate},
    language::Queryable,
    node::{ConsecutiveNodes, Node},
    query::MetavariableId,
    tree::TreeView,
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

#[derive(Debug)]
pub struct MatchedItem<'tree> {
    pub area: ConsecutiveNodes<'tree>,
    pub captures: HashMap<MetavariableId, CaptureItem<'tree>>,
}

impl<'tree> MatchedItem<'tree> {
    pub fn capture_of(&self, id: &MetavariableId) -> Option<&CaptureItem> {
        self.captures.get(id)
    }

    pub fn satisfies_all<T: Queryable + 'static>(
        &self,
        constraints: &[Constraint<T>],
    ) -> Result<bool> {
        for c in constraints {
            if !self.satisfies(c)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn satisfies<T: Queryable + 'static>(&self, constraint: &Constraint<T>) -> Result<bool> {
        let captured_item = self.capture_of(&constraint.target);
        match &constraint.predicate {
            Predicate::MatchQuery(q) => {
                if captured_item.is_none() {
                    return Ok(false);
                }
                let captured_item = captured_item.unwrap();
                match captured_item {
                    CaptureItem::Empty => Ok(false),
                    CaptureItem::Literal(_) => Err(anyhow::anyhow!(
                        "match-query predicate for string literals is not supported"
                    )),
                    CaptureItem::Nodes(n) => Ok(n.as_vec().iter().any(|node| {
                        let ptree = TreeView::<T>::from((*node).clone());
                        !ptree.matches(&q.into()).count() == 0
                    })),
                }
            }
            Predicate::NotMatchQuery(q) => {
                if captured_item.is_none() {
                    return Ok(true);
                }
                let captured_item = captured_item.unwrap();
                match captured_item {
                    CaptureItem::Empty => Ok(true),
                    CaptureItem::Literal(_) => Err(anyhow::anyhow!(
                        "match-query predicate for string literals is not supported"
                    )),
                    CaptureItem::Nodes(n) => Ok(n.as_vec().iter().all(|node| {
                        let ptree = TreeView::<T>::from((*node).clone());
                        ptree.matches(&q.into()).count() == 0
                    })),
                }
            }

            Predicate::MatchRegex(r) => {
                if captured_item.is_none() {
                    return Ok(false);
                }
                Ok(r.is_match(captured_item.unwrap().as_str()))
            }
            Predicate::NotMatchRegex(r) => {
                if captured_item.is_none() {
                    return Ok(true);
                }
                Ok(!r.is_match(captured_item.unwrap().as_str()))
            }
        }
    }
}
