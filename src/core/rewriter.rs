mod builder;
mod item;
mod node;
mod option;
use anyhow::Result;
pub use option::RewriteOption;

use super::{
    language::Queryable, matcher::MatchedItem, node::CSTNode, source::Code, tree::CSTView,
};

impl<T> Code<T>
where
    T: Queryable,
{
    pub fn rewrite<'tree>(
        self,
        view: &'tree CSTView<'tree, T>,
        item: &MatchedItem<'tree, CSTNode<'tree>>,
        roption: RewriteOption<T>,
    ) -> Result<Self> {
        let current_code = self.as_str().as_bytes();

        let before = self.string_between(0, item.area.start_byte())?;

        let snippet = roption.to_string_with(view, &item.captures)?;

        let after = self.string_between(
            item.area.end_byte().min(current_code.len()),
            current_code.len(),
        )?;

        Ok(Code::from(format!("{}{}{}", before, snippet, after)))
    }

    #[inline]
    pub fn string_between(&self, start: usize, end: usize) -> Result<String> {
        let source = self.as_str().as_bytes();
        Ok(String::from_utf8(source[start..end].to_vec())?)
    }
}
