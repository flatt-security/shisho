use crate::{
    code::Code, language::Queryable, matcher::MatchedItem, pattern::Pattern, query::MetavariableId,
};
use anyhow::{anyhow, Result};
use std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

pub struct AutofixPattern<'a, T>
where
    T: Queryable,
{
    pub raw_pattern: &'a [u8],
    tree: tree_sitter::Tree,
    _marker: PhantomData<T>,
}

impl<'a, T> AutofixPattern<'a, T>
where
    T: Queryable,
{
    pub fn to_patched_snippet<'tree>(&self, item: &'tree MatchedItem) -> Result<String> {
        let mut cursor = self.tree.walk();
        if !to_deepest_leaf(&mut cursor) {
            return Err(anyhow!("failed to initialize tree cursor"));
        }

        let mut text = "".to_string();
        let mut last_consumed_byte: usize = 0;
        while {
            let node = cursor.node();
            match node.kind() {
                "shisho_ellipsis" => {
                    // ignore
                    last_consumed_byte = node.end_byte();
                }
                "shisho_metavariable" => {
                    // ensure shisho_metavariable has only one child
                    let children: Vec<tree_sitter::Node> = {
                        let mut cursor = node.walk();
                        node.named_children(&mut cursor).collect()
                    };
                    if children.len() != 1 {
                        return Err(anyhow!("shisho_metavariable should have exactly one child (shisho_metavariable_name), but there are {} children", node.child_count()));
                    }

                    // extract child and get shisho_metavariable_name
                    // TODO (y0n3uchy): refactor these lines by introducing appropriate abstraction?
                    let child = children[0];
                    let id = match child.kind() {
                        "shisho_metavariable_name" => {
                            MetavariableId(child.utf8_text(self.raw_pattern).unwrap().to_string())
                        }
                        "shisho_metavariable_ellipsis_name" => {
                            let child = child.named_child(0).ok_or(anyhow!(
                                "failed to get shisho_metavariable_ellipsis_name child"
                            ))?;
                            MetavariableId(child.utf8_text(self.raw_pattern).unwrap().to_string())
                        }
                        _ => {
                            return Err(anyhow!("shisho_metavariable should have exactly one child (shisho_metavariable_name), but the child was {}", child.kind()));
                        }
                    };
                    let value = item
                        .metavariable_string(id)
                        .ok_or(anyhow!("metavariable not found"))?;

                    text = text
                        + String::from_utf8(
                            self.raw_pattern[(last_consumed_byte)..node.start_byte()].to_vec(),
                        )
                        .unwrap()
                        .as_str()
                        + value;
                    last_consumed_byte = node.end_byte();
                }
                _ => {
                    text = text
                        + String::from_utf8(
                            self.raw_pattern[(last_consumed_byte)..node.end_byte()].to_vec(),
                        )
                        .unwrap()
                        .as_str();
                    last_consumed_byte = node.end_byte();
                }
            }
            to_next_leaf(&mut cursor)
        } {}

        Ok(text
            + String::from_utf8(
                self.raw_pattern[(last_consumed_byte)..(self.raw_pattern.len())].to_vec(),
            )
            .unwrap()
            .as_str())
    }
}

fn to_deepest_leaf<'a>(cursor: &'a mut tree_sitter::TreeCursor) -> bool {
    while {
        match cursor.node().kind() {
            "shisho_ellipsis" | "shisho_metavariable" => false,
            _ => cursor.goto_first_child(),
        }
    } {}
    true
}

fn to_next_leaf<'a>(cursor: &'a mut tree_sitter::TreeCursor) -> bool {
    if cursor.goto_next_sibling() {
        to_deepest_leaf(cursor)
    } else {
        if cursor.goto_parent() {
            to_next_leaf(cursor)
        } else {
            // now the cursor points to the root node
            false
        }
    }
}

impl<'a, T> TryFrom<&'a str> for AutofixPattern<'a, T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let tree = Pattern::<T>::new(value).to_tstree()?;
        Ok(AutofixPattern {
            tree,
            raw_pattern: value.as_bytes(),
            _marker: PhantomData,
        })
    }
}

pub trait Transformable<T>
where
    T: Queryable,
    Self: Sized,
{
    fn transform<'a, P>(self, p: P, item: MatchedItem) -> Result<Self>
    where
        P: TryInto<AutofixPattern<'a, T>, Error = anyhow::Error>,
        P: 'a,
    {
        let query = p.try_into()?;
        self.transform_with_query(query, item)
    }

    fn transform_with_query(self, query: AutofixPattern<T>, item: MatchedItem) -> Result<Self>;
}

impl<T> Transformable<T> for Code<T>
where
    T: Queryable,
{
    fn transform_with_query(self, query: AutofixPattern<T>, item: MatchedItem) -> Result<Self> {
        let snippet = query.to_patched_snippet(&item)?;

        let start = item.top.start_byte();
        let end = item.top.end_byte();

        let current_code = self.as_str().as_bytes();
        let before = String::from_utf8(current_code[0..start].to_vec())?;
        let after = String::from_utf8(current_code[end..current_code.len()].to_vec())?;

        println!("code: {}", self.as_str());

        println!("before: {}", before);
        println!("snippet: {}", snippet);
        println!("after ({}-{}): {}", end, current_code.len(), after);

        Ok(Code::new(format!("{}{}{}", before, snippet, after)))
    }
}
