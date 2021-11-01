use anyhow::Result;

use crate::core::{
    language::Queryable,
    matcher::CaptureMap,
    node::Node,
    pattern::PatternView,
    ruleset::filter::{PatternWithFilters, RewriteFilter},
};

use super::builder::SnippetBuilder;

#[derive(Debug)]
pub struct RewriteOption<'a, T>
where
    T: Queryable,
{
    pview: PatternView<'a, T>,
    filters: &'a Vec<RewriteFilter<T>>,
}

impl<'a, T> RewriteOption<'a, T>
where
    T: Queryable,
{
    pub fn to_string_with<'tree>(
        &'a self,
        captures: &'tree CaptureMap<'tree, Node<'tree>>,
    ) -> Result<String> {
        let segment = SnippetBuilder::<T>::new(&self.pview, captures)
            .apply_filters(self.filters)?
            .build()?;

        Ok(segment.body)
    }
}

impl<'a, T> From<&'a PatternWithFilters<T>> for RewriteOption<'a, T>
where
    T: Queryable,
{
    fn from(pwf: &'a PatternWithFilters<T>) -> Self {
        Self {
            pview: PatternView::from(&pwf.pattern),
            filters: &pwf.filters,
        }
    }
}

impl<T> PatternWithFilters<T>
where
    T: Queryable,
{
    pub fn as_roption(&'_ self) -> RewriteOption<'_, T> {
        self.into()
    }
}
