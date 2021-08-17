use crate::{
    language::Queryable,
    pattern::Pattern,
    tree::{ShishoOperation, TreeLike},
};
use anyhow::{anyhow, Result};
use std::{
    array::IntoIter,
    collections::HashMap,
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

pub const TOP_CAPTURE_ID_PREFIX: &str = "TOP-";

pub const SHISHO_NODE_METAVARIABLE_NAME: &str = "shisho_metavariable_name";
pub const SHISHO_NODE_METAVARIABLE: &str = "shisho_metavariable";
pub const SHISHO_NODE_ELLIPSIS_METAVARIABLE: &str = "shisho_ellipsis_metavariable";
pub const SHISHO_NODE_ELLIPSIS: &str = "shisho_ellipsis";

#[derive(Debug, PartialEq)]
pub struct Query<T>
where
    T: Queryable,
{
    pub metavariables: MetavariableTable,

    query: tree_sitter::Query,
    _marker: PhantomData<T>,
}

impl<T> Query<T>
where
    T: Queryable,
{
    pub fn new(query: tree_sitter::Query, metavariables: MetavariableTable) -> Self {
        Query {
            query,
            metavariables,
            _marker: PhantomData,
        }
    }

    pub fn get_cid_mvid_map(&self) -> HashMap<CaptureId, MetavariableId> {
        self.metavariables
            .iter()
            .map(|(k, v)| v.iter().map(move |id| (id.clone(), k.clone())))
            .flatten()
            .collect::<HashMap<CaptureId, MetavariableId>>()
    }
}

impl<T> AsRef<tree_sitter::Query> for Query<T>
where
    T: Queryable,
{
    fn as_ref(&self) -> &tree_sitter::Query {
        &self.query
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct CaptureId(pub String);

impl AsRef<str> for CaptureId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetavariableId(pub String);

pub type MetavariableTable = HashMap<MetavariableId, Vec<CaptureId>>;

impl<T> TryFrom<&str> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let p = Pattern::from(value);
        p.try_into()
    }
}

impl<T> TryFrom<Pattern<'_, T>> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: Pattern<'_, T>) -> Result<Self, Self::Error> {
        let tsq = TSQueryString::try_from(value)?;

        let tsquery = tree_sitter::Query::new(T::target_language(), &tsq.query_string)?;

        Ok(Query::new(tsquery, tsq.metavariables))
    }
}

#[derive(Debug, PartialEq)]
pub struct TSQueryString<T>
where
    T: Queryable,
{
    pub metavariables: MetavariableTable,
    pub query_string: String,

    _marker: PhantomData<T>,
}

impl<T> TSQueryString<T>
where
    T: Queryable,
{
    pub fn new(query_string: String, metavariables: MetavariableTable) -> Self {
        TSQueryString {
            query_string,
            metavariables,
            _marker: PhantomData,
        }
    }
}

impl<T> TryFrom<&str> for TSQueryString<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let p = Pattern::from(value);
        p.try_into()
    }
}

impl<T> TryFrom<Pattern<'_, T>> for TSQueryString<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: Pattern<'_, T>) -> Result<Self, Self::Error> {
        let query_tree = value.to_tstree()?;

        let processor = RawQueryProcessor::<T>::from(value.as_ref());
        let (child_query_strings, metavariables) =
            processor.convert_nodes(T::extract_query_nodes(&query_tree))?;

        let query_string = format!(
            "({} {})",
            child_query_strings
                .into_iter()
                .enumerate()
                .map(|(index, query)| format!("{} @{}{}", query, TOP_CAPTURE_ID_PREFIX, index))
                .collect::<Vec<String>>()
                .join(" . "),
            metavariables
                .to_query_constraints()
                .into_iter()
                .collect::<Vec<String>>()
                .join(" "),
        );

        Ok(TSQueryString {
            query_string,
            metavariables,
            _marker: PhantomData,
        })
    }
}

pub trait ToQueryConstraintString {
    fn to_query_constraints(&self) -> Vec<String>;
}

impl ToQueryConstraintString for MetavariableTable {
    fn to_query_constraints(&self) -> Vec<String> {
        self.into_iter()
            .filter_map(|(_k, mvs)| {
                let ids: Vec<String> = mvs.iter().map(|id| id.0.clone()).collect();
                if ids.len() <= 1 {
                    None
                } else {
                    let first_capture_id = ids[0].clone();
                    Some(
                        ids.into_iter()
                            .skip(1)
                            .map(|id| format!("(#eq? @{} @{})", first_capture_id, id))
                            .collect::<Vec<String>>()
                            .join(" "),
                    )
                }
            })
            .collect()
    }
}

#[derive(Debug, PartialEq)]
struct RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    raw_bytes: &'a [u8],
    _marker: PhantomData<T>,
}

impl<'a, T> From<&'a [u8]> for RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    fn from(value: &'a [u8]) -> Self {
        RawQueryProcessor {
            raw_bytes: value,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    fn convert_node<'b>(&self, node: tree_sitter::Node<'b>) -> Result<TSQueryString<T>>
    where
        'a: 'b,
    {
        match node.kind() {
            SHISHO_NODE_ELLIPSIS => self.convert_ellipsis_node(&node),
            SHISHO_NODE_ELLIPSIS_METAVARIABLE => self.convert_ellipsis_metavariable_node(&node),
            SHISHO_NODE_METAVARIABLE => self.convert_metavariable_node(&node),
            _ => {
                let children: Vec<tree_sitter::Node> = {
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor).collect()
                };

                if children.len() > 0 {
                    // this is not leaf node. it shoulde be under a constraint on the value.
                    let (child_query_strings, metavariables) = self.convert_nodes(children)?;
                    Ok(TSQueryString::new(
                        format!(
                            r#"({} . {} .)"#,
                            node.kind(),
                            child_query_strings
                                .into_iter()
                                .collect::<Vec<String>>()
                                .join(" . "),
                        ),
                        metavariables,
                    ))
                } else {
                    // this is leaf node. it should be under a constraint on the value.
                    Ok(TSQueryString::new(
                        format!(
                            r#"(({}) @{} (#eq? @{} "{}"))"#,
                            node.kind(),
                            node.id(),
                            node.id(),
                            self.node_as_str(&node).replace("\"", "\\\"")
                        ),
                        MetavariableTable::new(),
                    ))
                }
            }
        }
    }

    fn convert_ellipsis_node<'b>(&self, _node: &tree_sitter::Node<'b>) -> Result<TSQueryString<T>>
    where
        'a: 'b,
    {
        Ok(TSQueryString::new(
            "((_) _?)*  . ((_) _?)?".into(),
            MetavariableTable::new(),
        ))
    }

    fn convert_ellipsis_metavariable_node<'b>(
        &self,
        node: &tree_sitter::Node<'b>,
    ) -> Result<TSQueryString<T>>
    where
        'a: 'b,
    {
        let (variable_name, capture_id) = self.extract_vname_from_node(node).ok_or(anyhow!(
            "{} did not have {}",
            SHISHO_NODE_METAVARIABLE,
            SHISHO_NODE_METAVARIABLE_NAME
        ))?;

        // NOTE: into_iter() can't be used here https://github.com/rust-lang/rust/pull/84147
        let metavariables = IntoIter::new([(
            MetavariableId(variable_name.into()),
            vec![capture_id.clone()],
        )])
        .collect::<MetavariableTable>();

        Ok(TSQueryString::new(
            format!(
                "((_) _?)* @{} . ((_) _?)? @{}",
                capture_id.as_ref(),
                capture_id.as_ref()
            ),
            metavariables,
        ))
    }

    fn convert_metavariable_node<'b>(
        &self,
        node: &tree_sitter::Node<'b>,
    ) -> Result<TSQueryString<T>>
    where
        'a: 'b,
    {
        let (variable_name, capture_id) = self.extract_vname_from_node(node).ok_or(anyhow!(
            "{} did not have {}",
            SHISHO_NODE_METAVARIABLE,
            SHISHO_NODE_METAVARIABLE_NAME
        ))?;

        // NOTE: into_iter() can't be used here https://github.com/rust-lang/rust/pull/84147
        let metavariables = IntoIter::new([(
            MetavariableId(variable_name.into()),
            vec![capture_id.clone()],
        )])
        .collect::<MetavariableTable>();

        Ok(TSQueryString::new(
            format!("((_) @{})", capture_id.as_ref()),
            metavariables,
        ))
    }

    fn convert_nodes<'b>(
        &self,
        nodes: Vec<tree_sitter::Node<'b>>,
    ) -> Result<(Vec<String>, MetavariableTable)>
    where
        'a: 'b,
    {
        let mut child_queries = vec![];
        let mut metavariables: MetavariableTable = MetavariableTable::new();

        for node in nodes {
            match self.convert_node(node) {
                Ok(TSQueryString {
                    query_string,
                    metavariables: child_metavariables,
                    ..
                }) => {
                    child_queries.push(query_string);
                    for (key, value) in child_metavariables {
                        if let Some(mv) = metavariables.get_mut(&key) {
                            mv.extend(value);
                        } else {
                            metavariables.insert(key, value);
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Ok((child_queries, metavariables))
    }
}

impl<'a, T> TreeLike<'a> for RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    fn node_as_str(&self, node: &tree_sitter::Node) -> &'a str {
        node.utf8_text(self.raw_bytes).unwrap()
    }
}
