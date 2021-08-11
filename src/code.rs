use crate::language::Queryable;
use std::marker::PhantomData;

#[derive(Clone)]
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
    pub fn as_str<'a>(&'a self) -> &'a str {
        self.code.as_str()
    }
}

impl<T> AsRef<str> for Code<T>
where
    T: Queryable,
{
    fn as_ref(&self) -> &str {
        self.code.as_str()
    }
}

impl<T, C> From<T> for Code<C>
where
    T: Into<String>,
    C: Queryable,
{
    fn from(code: T) -> Self {
        Self {
            code: code.into(),
            _marker: PhantomData,
        }
    }
}
