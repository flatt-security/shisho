use anyhow::Result;

use crate::{
    code::Code,
    language::Queryable,
    matcher::MatchedItem,
    query::{Pattern, Query},
};

pub type AutofixQuery<T> = Query<T>;

pub trait Transformable<T>
where
    T: Queryable,
    Self: Sized,
{
    fn transform_with_pattern(self, pattern: &str, item: MatchedItem) -> Result<Self> {
        let query = Pattern::<T>::new(pattern).to_query()?;
        self.transform_with_query(query, item)
    }

    fn transform_with_query(self, query: AutofixQuery<T>, item: MatchedItem) -> Result<Self>;
}
