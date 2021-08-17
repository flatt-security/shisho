use crate::{
    code::Code, language::Queryable, matcher::MatchedItem, pattern::Pattern, query::MetavariableId,
};
use anyhow::{anyhow, Result};
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

pub struct AutofixPattern<'pattern, T>
where
    T: Queryable,
{
    raw: &'pattern [u8],
    tree: tree_sitter::Tree,
    _marker: PhantomData<T>,
}

impl<'a, T> AutofixPattern<'a, T>
where
    T: Queryable,
{
    pub fn to_patched_snippet<'tree>(&self, item: &'tree MatchedItem) -> Result<String> {
        self.to_patched_snippet_intl(item, &self.tree.root_node())
    }

    fn to_patched_snippet_intl<'tree>(
        &self,
        item: &'tree MatchedItem,
        node: &'tree tree_sitter::Node,
    ) -> Result<String> {
        match node.kind() {
            "shisho_metavariable" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.kind() == "shisho_metavariable_name" {
                        let id = MetavariableId(self.str_from_node(&child).to_string());
                        let value = item
                            .get_captured_string(&id)
                            .ok_or(anyhow!("metavariable not found"))?;
                        return Ok(value.into());
                    }
                }
                return Err(anyhow!("shisho_metavariable should have exactly one child (shisho_metavariable_name), but there are {} children", node.child_count()));
            }
            "shisho_ellipsis" | "shisho_ellipsis_metavariable" => {
                return Err(anyhow!(
                    "cannot use ellipsis operator inside the transformation query"
                ));
            }
            _ if node.child_count() == 0 => Ok(self.str_from_node(node).into()),
            _ => {
                let mut cursor = node.walk();

                let mut snippet: String = "".into();
                let mut end: usize = node.start_byte();
                for child in node.children(&mut cursor) {
                    let child_string = self.to_patched_snippet_intl(item, &child)?;
                    snippet = snippet
                        + self.str_from_range(end, child.start_byte()).as_str()
                        + child_string.as_str();
                    end = child.end_byte();
                }
                snippet = snippet + self.str_from_range(end, node.end_byte()).as_str();

                Ok(snippet)
            }
        }
    }

    fn str_from_node(&self, n: &tree_sitter::Node) -> &str {
        n.utf8_text(self.raw).unwrap()
    }

    fn str_from_range(&self, start: usize, end: usize) -> String {
        String::from_utf8(self.raw[start..end].to_vec()).unwrap()
    }
}

impl<'pattern, T> AsRef<tree_sitter::Tree> for AutofixPattern<'pattern, T>
where
    T: Queryable,
{
    fn as_ref(&self) -> &tree_sitter::Tree {
        &self.tree
    }
}

impl<'pattern, T> AsRef<[u8]> for AutofixPattern<'pattern, T>
where
    T: Queryable,
{
    fn as_ref(&self) -> &[u8] {
        &self.raw
    }
}

impl<'a, T> TryFrom<&'a str> for AutofixPattern<'a, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let tree = Pattern::<T>::from(value).to_tstree()?;
        Ok(Self {
            tree,
            raw: value.as_bytes(),
            _marker: PhantomData,
        })
    }
}

pub trait Transformable<T>
where
    T: Queryable,
    Self: Sized,
{
    fn transform<'a, P>(self, item: MatchedItem, p: P) -> Result<Self>
    where
        P: TryInto<AutofixPattern<'a, T>, Error = anyhow::Error>,
        P: 'a,
    {
        let query = p.try_into()?;
        self.transform_with_query(item, query)
    }

    fn transform_with_query(self, item: MatchedItem, query: AutofixPattern<T>) -> Result<Self>;
}

impl<T> Transformable<T> for Code<T>
where
    T: Queryable,
{
    fn transform_with_query(self, item: MatchedItem, query: AutofixPattern<T>) -> Result<Self> {
        let current_code = self.as_str().as_bytes();

        let before_snippet = String::from_utf8(current_code[0..item.top.start_byte()].to_vec())?;
        let snippet = query.to_patched_snippet(&item)?;
        let after_snippet =
            String::from_utf8(current_code[item.top.end_byte()..current_code.len()].to_vec())?;

        Ok(Code::from(format!(
            "{}{}{}",
            before_snippet, snippet, after_snippet
        )))
    }
}
