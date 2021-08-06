use crate::{
    language::Queryable,
    matcher::MatchedItem,
    transform::{AutofixQuery, Transformable},
};
use std::marker::PhantomData;

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

    pub fn into_string(self) -> String {
        self.code
    }
}

impl<T, C> From<T> for Code<C>
where
    T: Into<String>,
    C: Queryable,
{
    fn from(code: T) -> Self {
        Code::new(code)
    }
}

impl<T> Transformable<T> for Code<T>
where
    T: Queryable,
{
    fn transform_with_query(self, query: AutofixQuery<T>, item: MatchedItem) -> Result<Self> {
        Ok(self)
    }
}
