use std::marker::PhantomData;

use crate::{language::Queryable, matcher::MatchedItem, query::Query};
use thiserror::Error;

pub struct Code<L>
where
    L: Queryable,
{
    code: String,
    _marker: PhantomData<L>,
}

impl<T> Code<T>
where
    T: Queryable,
{
    pub fn new(code: impl Into<String>) -> Self {
        Code {
            code: code.into(),
            _marker: PhantomData,
        }
    }

    pub fn into_string(self) -> Self {
        self.code
    }
}
