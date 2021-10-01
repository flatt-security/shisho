mod snippet;

use crate::core::{
    language::Queryable, matcher::MatchedItem, node::RootNode, pattern::Pattern, source::Code,
};
use anyhow::Result;

use self::snippet::SnippetBuilder;

pub struct RewriteOption<'a, T>
where
    T: Queryable,
{
    pub root_node: RootNode<'a>,
    pattern: &'a Pattern<T>,
}

impl<'a, T> RewriteOption<'a, T>
where
    T: Queryable,
{
    pub fn to_rewritten_snippet<'tree>(&self, item: &'tree MatchedItem) -> Result<String> {
        Ok(SnippetBuilder::new(self, item)
            .from_root(&self.root_node)?
            .body)
    }
}

impl<'a, T> From<&'a Pattern<T>> for RewriteOption<'a, T>
where
    T: Queryable + 'static,
{
    fn from(pattern: &'a Pattern<T>) -> Self {
        let root_node = pattern.to_root_node();
        Self { pattern, root_node }
    }
}

impl<T> Pattern<T>
where
    T: Queryable + 'static,
{
    pub fn as_autofix(&'_ self) -> RewriteOption<'_, T> {
        self.into()
    }
}

pub trait Rewritable<T>
where
    T: Queryable + 'static,
    Self: Sized,
{
    fn to_rewritten_form<'a, P>(self, item: &MatchedItem, p: P) -> Result<Self>
    where
        P: Into<RewriteOption<'a, T>>,
    {
        let roption = p.into();
        self.to_rewritten_form_intl(item, roption)
    }

    fn to_rewritten_form_intl(self, item: &MatchedItem, roption: RewriteOption<T>) -> Result<Self>;
}

impl<T> Rewritable<T> for Code<T>
where
    T: Queryable + 'static,
{
    fn to_rewritten_form_intl(self, item: &MatchedItem, roption: RewriteOption<T>) -> Result<Self> {
        let current_code = self.as_str().as_bytes();

        let before_snippet = String::from_utf8(current_code[0..item.area.start_byte()].to_vec())?;
        let snippet = roption.to_rewritten_snippet(item)?;
        let after_snippet = String::from_utf8(
            current_code[item.area.end_byte().min(current_code.len())..current_code.len()].to_vec(),
        )?;

        Ok(Code::from(format!(
            "{}{}{}",
            before_snippet, snippet, after_snippet
        )))
    }
}
