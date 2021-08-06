use std::convert::TryInto;

use anyhow::Result;

use crate::{language::Queryable, matcher::MatchedItem, query::Query};

pub type AutofixQuery<T> = Query<T>;

pub trait Transformable<T>
where
    T: Queryable,
    Self: Sized,
{
    fn transform<P>(self, p: P, item: MatchedItem) -> Result<Self>
    where
        P: TryInto<AutofixQuery<T>, Error = anyhow::Error>,
    {
        let query = p.try_into()?;
        self.transform_with_query(query, item)
    }

    fn transform_with_query(self, query: AutofixQuery<T>, item: MatchedItem) -> Result<Self>;
}
