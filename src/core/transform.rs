use crate::core::{
    language::Queryable, matcher::MatchedItem, pattern::Pattern, query::MetavariableId,
    source::Code, tree::TreeVisitor,
};
use anyhow::{anyhow, Result};
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

use super::{
    node::{Node, RootNode},
    source::NormalizedSource,
};

pub struct AutofixItem<T>
where
    T: Queryable,
{
    pattern: Pattern<T>,
    _marker: PhantomData<T>,
}

impl<T> AutofixItem<T>
where
    T: Queryable,
{
    pub fn root_node(&'_ self) -> RootNode<'_> {
        self.pattern.root_node()
    }

    pub fn to_patched_snippet<'tree>(&self, item: &'tree MatchedItem) -> Result<String> {
        let processor = PatchProcessor {
            autofix: self,
            item,
        };

        let patched_item = T::unwrap_root(&self.root_node())
            .iter()
            .filter(|x| !T::is_skippable(x))
            .map(|node| processor.handle_node(node))
            .collect::<Result<Vec<PatchedItem>>>()?;

        Ok(patched_item
            .into_iter()
            .map(|item| item.body)
            .collect::<Vec<String>>()
            .join(""))
    }
}

impl<T> TryFrom<&str> for AutofixItem<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let source = NormalizedSource::from(value);
        source.try_into()
    }
}

impl<T> TryFrom<NormalizedSource> for AutofixItem<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(source: NormalizedSource) -> Result<Self, Self::Error> {
        let pattern = Pattern::<T>::try_from(source)?;
        Ok(Self {
            pattern,
            _marker: PhantomData,
        })
    }
}

pub struct PatchProcessor<'pattern, T>
where
    T: Queryable,
{
    autofix: &'pattern AutofixItem<T>,
    item: &'pattern MatchedItem<'pattern>,
}

pub struct PatchedItem {
    pub body: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl<'tree, T> TreeVisitor<'tree, T> for PatchProcessor<'tree, T>
where
    T: Queryable,
{
    type Output = PatchedItem;

    fn walk_metavariable(
        &self,
        node: &Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error> {
        let id = MetavariableId(variable_name.into());
        let value = self
            .item
            .capture_of(&id)
            .map(|x| x.as_str())
            .unwrap_or_default();
        // .ok_or(anyhow!("metavariable not found"))?;
        Ok(PatchedItem {
            body: value.into(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    fn walk_ellipsis(&self, _node: &Node) -> Result<Self::Output, anyhow::Error> {
        Err(anyhow!(
            "cannot use ellipsis operator inside the transformation query"
        ))
    }

    fn walk_ellipsis_metavariable(
        &self,
        _node: &Node,
        _variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error> {
        Err(anyhow!(
            "cannot use ellipsis operator inside the transformation query"
        ))
    }

    fn walk_leaf_named_node(&self, node: &Node) -> Result<Self::Output, anyhow::Error> {
        Ok(PatchedItem {
            body: node.as_str().into(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    fn walk_leaf_unnamed_node(&self, node: &Node) -> Result<Self::Output, anyhow::Error> {
        Ok(PatchedItem {
            body: node.as_str().into(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    fn flatten_intermediate_node(
        &self,
        node: &Node,
        children: Vec<Self::Output>,
    ) -> Result<Self::Output, anyhow::Error> {
        let mut body: String = "".into();
        let mut end: usize = node.start_byte();

        for child in children {
            body += self
                .autofix
                .pattern
                .string_between(end, child.start_byte)?
                .as_str();
            body += child.body.as_str();
            end = child.end_byte;
        }
        body += self
            .autofix
            .pattern
            .string_between(end, node.end_byte())?
            .as_str();

        Ok(PatchedItem {
            body,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }
}

pub trait Transformable<T>
where
    T: Queryable,
    Self: Sized,
{
    fn transform<P>(self, item: &MatchedItem, p: P) -> Result<Self>
    where
        P: TryInto<AutofixItem<T>, Error = anyhow::Error>,
    {
        let query = p.try_into()?;
        self.transform_with_query(item, query)
    }

    fn transform_with_query(self, item: &MatchedItem, query: AutofixItem<T>) -> Result<Self>;
}

impl<T> Transformable<T> for Code<T>
where
    T: Queryable,
{
    fn transform_with_query(self, item: &MatchedItem, query: AutofixItem<T>) -> Result<Self> {
        let current_code = self.as_str().as_bytes();

        let before_snippet = String::from_utf8(current_code[0..item.area.start_byte()].to_vec())?;
        let snippet = query.to_patched_snippet(item)?;
        let after_snippet = String::from_utf8(
            current_code[item.area.end_byte().min(current_code.len())..current_code.len()].to_vec(),
        )?;

        Ok(Code::from(format!(
            "{}{}{}",
            before_snippet, snippet, after_snippet
        )))
    }
}
