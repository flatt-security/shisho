use crate::core::{
    language::Queryable,
    matcher::MatchedItem,
    node::{Node, RootNode},
    pattern::Pattern,
    query::MetavariableId,
    source::Code,
    tree::TreeVisitor,
};
use anyhow::{anyhow, Result};

pub struct AutofixItem<'a, T>
where
    T: Queryable,
{
    pub root_node: RootNode<'a>,
    pattern: &'a Pattern<T>,
}

impl<'a, T> AutofixItem<'a, T>
where
    T: Queryable,
{
    pub fn to_patched_snippet<'tree>(&self, item: &'tree MatchedItem) -> Result<String> {
        let processor = PatchProcessor {
            autofix: self,
            item,
        };

        let pitems = T::unwrap_root(&self.root_node)
            .iter()
            .map(|node| processor.handle_node(node))
            .collect::<Result<Vec<PatchedItem>>>()?;

        Ok(processor
            .flatten_patched_item(0, self.root_node.as_node().end_byte(), pitems)?
            .body)
    }
}

impl<'a, T> From<&'a Pattern<T>> for AutofixItem<'a, T>
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
    pub fn as_autofix(&'_ self) -> AutofixItem<'_, T> {
        self.into()
    }
}

pub struct PatchProcessor<'pattern, T>
where
    T: Queryable,
{
    autofix: &'pattern AutofixItem<'pattern, T>,
    item: &'pattern MatchedItem<'pattern>,
}

#[derive(Debug)]
pub struct PatchedItem {
    pub body: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl<'tree, T> PatchProcessor<'tree, T>
where
    T: Queryable,
{
    fn flatten_patched_item(
        &self,
        start_byte: usize,
        end_byte: usize,
        children: Vec<PatchedItem>,
    ) -> Result<PatchedItem, anyhow::Error> {
        let mut body: String = "".into();
        let mut end: usize = start_byte;

        for child in children {
            body += self
                .autofix
                .pattern
                .string_between(end, child.start_byte)?
                .as_str();
            body += child.body.as_str();
            end = child.end_byte;
        }
        body += self.autofix.pattern.string_between(end, end_byte)?.as_str();

        Ok(PatchedItem {
            body,
            start_byte: start_byte,
            end_byte: end_byte,
        })
    }
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
            // TODO: should be handle this None like `.ok_or(anyhow!("metavariable not found"))?;`?
            .unwrap_or_default();
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
        self.flatten_patched_item(node.start_byte(), node.end_byte(), children)
    }
}

pub trait Transformable<T>
where
    T: Queryable + 'static,
    Self: Sized,
{
    fn transform<'a, P>(self, item: &MatchedItem, p: P) -> Result<Self>
    where
        P: Into<AutofixItem<'a, T>>,
    {
        let query = p.into();
        self.transform_with_query(item, query)
    }

    fn transform_with_query(self, item: &MatchedItem, query: AutofixItem<T>) -> Result<Self>;
}

impl<T> Transformable<T> for Code<T>
where
    T: Queryable + 'static,
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
