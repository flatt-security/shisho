use crate::tree::TSTreeVisitor;
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
        let processor = PatchProcessor {
            pattern: self,
            item,
        };
        let patched_item = T::extract_query_nodes(&self.tree)
            .into_iter()
            .map(|node| processor.handle_node(node))
            .collect::<Result<Vec<PatchedItem>>>()?;
        Ok(patched_item
            .into_iter()
            .map(|item| item.body)
            .collect::<Vec<String>>()
            .join(""))
    }
}

pub struct PatchProcessor<'tree, T>
where
    T: Queryable,
{
    pattern: &'tree AutofixPattern<'tree, T>,
    item: &'tree MatchedItem<'tree>,
}

impl<'tree, T> PatchProcessor<'tree, T>
where
    T: Queryable,
{
    fn str_from_range(&self, start: usize, end: usize) -> String {
        String::from_utf8(self.pattern.raw[start..end].to_vec()).unwrap()
    }
}

pub struct PatchedItem {
    pub body: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl<'tree, T> TSTreeVisitor<'tree> for PatchProcessor<'tree, T>
where
    T: Queryable,
{
    type Output = PatchedItem;

    fn walk_metavariable(
        &self,
        node: tree_sitter::Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error> {
        let id = MetavariableId(variable_name.into());
        let value = self
            .item
            .get_captured_string(&id)
            .ok_or(anyhow!("metavariable not found"))?;
        Ok(PatchedItem {
            body: value.into(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    fn walk_ellipsis(&self, _node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error> {
        Err(anyhow!(
            "cannot use ellipsis operator inside the transformation query"
        ))
    }

    fn walk_ellipsis_metavariable(
        &self,
        _node: tree_sitter::Node,
        _variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error> {
        Err(anyhow!(
            "cannot use ellipsis operator inside the transformation query"
        ))
    }

    fn walk_leaf_node(&self, node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error> {
        Ok(PatchedItem {
            body: self.node_as_str(&node).into(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    fn flatten_intermediate_node(
        &self,
        node: tree_sitter::Node,
        children: Vec<Self::Output>,
    ) -> Result<Self::Output, anyhow::Error> {
        let mut body: String = "".into();
        let mut end: usize = node.start_byte();

        for child in children {
            body = body + self.str_from_range(end, child.start_byte).as_str() + child.body.as_str();
            end = child.end_byte;
        }
        body = body + self.str_from_range(end, node.end_byte()).as_str();

        Ok(PatchedItem {
            body,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    fn node_as_str(&self, node: &tree_sitter::Node) -> &'tree str {
        node.utf8_text(self.pattern.raw).unwrap()
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
