use std::convert::TryFrom;

use anyhow::Result;

use crate::core::{
    language::Queryable,
    matcher::CaptureMap,
    node::CSTNode,
    pattern::Pattern,
    ruleset::filter::{PatternWithFilters, RewriteFilter},
    tree::CSTView,
};

use super::builder::SnippetBuilder;

#[derive(Debug)]
pub struct RewriteOption<'a, T>
where
    T: Queryable,
{
    pattern: Pattern<'a, T>,
    filters: &'a Vec<RewriteFilter<T>>,
}

impl<'a, T> RewriteOption<'a, T>
where
    T: Queryable,
{
    pub fn to_string_with<'tree>(
        &'a self,
        view: &'tree CSTView<'tree, T>,
        captures: &'tree CaptureMap<'tree, CSTNode<'tree>>,
    ) -> Result<String> {
        let segment = SnippetBuilder::<T>::new(&self.pattern, view, captures)
            .apply_filters(self.filters)?
            .build()?;

        Ok(segment.body)
    }
}

impl<'a, T> TryFrom<&'a PatternWithFilters<T>> for RewriteOption<'a, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &'a PatternWithFilters<T>) -> Result<Self, Self::Error> {
        let p = Pattern::<T>::try_from(&value.pattern)?;
        Ok(Self {
            pattern: p,
            filters: &value.filters,
        })
    }
}
