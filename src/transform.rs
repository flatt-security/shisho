use crate::tree::ShishoOperation;
use crate::{
    code::Code,
    language::Queryable,
    matcher::MatchedItem,
    pattern::Pattern,
    query::{
        MetavariableId, SHISHO_NODE_ELLIPSIS, SHISHO_NODE_ELLIPSIS_METAVARIABLE,
        SHISHO_NODE_METAVARIABLE, SHISHO_NODE_METAVARIABLE_NAME,
    },
    tree::TreeLike,
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
            SHISHO_NODE_METAVARIABLE => {
                let (variable_name, _) = self.extract_vname_from_node(node).ok_or(anyhow!(
                    "{} did not have {}",
                    SHISHO_NODE_METAVARIABLE,
                    SHISHO_NODE_METAVARIABLE_NAME,
                ))?;
                let id = MetavariableId(variable_name.into());
                let value = item
                    .get_captured_string(&id)
                    .ok_or(anyhow!("metavariable not found"))?;
                Ok(value.into())
            }
            SHISHO_NODE_ELLIPSIS | SHISHO_NODE_ELLIPSIS_METAVARIABLE => Err(anyhow!(
                "cannot use ellipsis operator inside the transformation query"
            )),
            _ if node.child_count() == 0 => Ok(self.node_as_str(node).into()),
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

    fn str_from_range(&self, start: usize, end: usize) -> String {
        String::from_utf8(self.raw[start..end].to_vec()).unwrap()
    }
}

impl<'a, T> TreeLike<'a> for AutofixPattern<'a, T>
where
    T: Queryable,
{
    fn node_as_str(&self, node: &tree_sitter::Node) -> &'a str {
        node.utf8_text(self.raw).unwrap()
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
