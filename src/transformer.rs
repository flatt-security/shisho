use crate::{code::Code, language::Queryable, matcher::MatchedItem, query::Query};

pub type AutofixQuery<T> = Query<T>;

pub trait Transformable<T>
where
    T: Queryable,
{
    fn apply(self, item: MatchedItem, q: AutofixQuery<T>) -> Self;
}

impl<T> Transformable<T> for Code<T>
where
    T: Queryable,
{
    fn apply(self, item: MatchedItem, q: AutofixQuery<T>) -> Self {
        self
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
