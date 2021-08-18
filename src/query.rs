use crate::{language::Queryable, pattern::Pattern, tree::TSTreeVisitor, util::Merge};
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

    fn try_from(value: &str) -> Result<Self, anyhow::Error> {
        let p = Pattern::from(value);
        p.try_into()
    }
}

impl<T> TryFrom<Pattern<'_, T>> for Query<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: Pattern<'_, T>) -> Result<Self, anyhow::Error> {
        let tsq = TSQueryString::try_from(value)?;

        let tsquery = tree_sitter::Query::new(T::target_language(), &tsq.query_string)
            .map_err(|_| anyhow!("failed to load the pattern; invalid pattern was given"))?;

        Ok(Query::new(tsquery, tsq.metavariables))
    }
}

#[derive(Debug, PartialEq, Default)]
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

    fn try_from(value: &str) -> Result<Self, anyhow::Error> {
        let p = Pattern::from(value);
        p.try_into()
    }
}

impl<T> TryFrom<Pattern<'_, T>> for TSQueryString<T>
where
    T: Queryable,
{
    type Error = anyhow::Error;

    fn try_from(value: Pattern<'_, T>) -> Result<Self, anyhow::Error> {
        let query_tree = value.to_tstree()?;

        let processor = RawQueryProcessor::<T>::from(value.as_ref());
        let queries = T::extract_query_nodes(&query_tree)?;
        let queries = queries
            .into_iter()
            .map(|node| processor.handle_node(node))
            .collect::<Result<Vec<TSQueryString<T>>>>()?;

        let (child_strings, child_metavariables): (Vec<String>, Vec<MetavariableTable>) = queries
            .into_iter()
            .map(|q| (q.query_string, q.metavariables))
            .unzip();

        let metavariables =
            child_metavariables
                .into_iter()
                .fold(MetavariableTable::new(), |mut acc, item| {
                    acc.merge(item);
                    acc
                });

        let query_string = format!(
            "({} {})",
            child_strings
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
        Ok(TSQueryString::new(query_string, metavariables))
    }
}

pub trait ToQueryConstraintString {
    fn to_query_constraints(&self) -> Vec<String>;
}

impl ToQueryConstraintString for MetavariableTable {
    fn to_query_constraints(&self) -> Vec<String> {
        self.into_iter()
            .filter_map(|(k, mvs)| {
                let ids: Vec<String> = mvs.iter().map(|id| id.0.clone()).collect();
                if ids.len() <= 1 || k == &MetavariableId("_".into()) {
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

impl<'a, T> TSTreeVisitor<'a, T> for RawQueryProcessor<'a, T>
where
    T: Queryable,
{
    type Output = TSQueryString<T>;

    fn walk_leaf_node(&self, node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error> {
        if node.is_named() {
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
        } else {
            let v = self
                .node_as_str(&node)
                .replace("\"", "\\\"")
                .replace("\n", "\\n");
            Ok(TSQueryString::new(
                if v == "" {
                    "".into()
                } else {
                    format!(r#""{}""#, v)
                },
                MetavariableTable::new(),
            ))
        }
    }

    fn walk_ellipsis(&self, _node: tree_sitter::Node) -> Result<Self::Output, anyhow::Error> {
        Ok(TSQueryString::new(
            "((_) _?)*  . ((_) _?)?".into(),
            MetavariableTable::new(),
        ))
    }

    fn walk_ellipsis_metavariable(
        &self,
        node: tree_sitter::Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error> {
        let capture_id = self.node_as_capture_id(&node);

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

    fn walk_metavariable(
        &self,
        node: tree_sitter::Node,
        variable_name: &str,
    ) -> Result<Self::Output, anyhow::Error> {
        let capture_id = self.node_as_capture_id(&node);

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

    fn node_as_str(&self, node: &tree_sitter::Node) -> &'a str {
        std::str::from_utf8(
            &self.raw_bytes[node.start_byte().min(self.raw_bytes.len())
                ..node.end_byte().min(self.raw_bytes.len())],
        )
        .unwrap()
    }

    fn flatten_intermediate_node(
        &self,
        node: tree_sitter::Node,
        children: Vec<Self::Output>,
    ) -> Result<Self::Output, anyhow::Error> {
        let mut child_queries = vec![];
        let mut metavariables: MetavariableTable = MetavariableTable::new();

        for TSQueryString {
            query_string,
            metavariables: child_metavariables,
            ..
        } in children
        {
            child_queries.push(query_string);
            metavariables.merge(child_metavariables);
        }

        let query = format!(
            r#"({} . {} .)"#,
            node.kind(),
            child_queries
                .into_iter()
                .filter(|x| x != "")
                .collect::<Vec<String>>()
                .join(" . "),
        );
        Ok(TSQueryString::new(query, metavariables))
    }
}

impl Merge for MetavariableTable {
    fn merge(&mut self, other: Self) {
        for (key, value) in other {
            if let Some(mv) = self.get_mut(&key) {
                mv.extend(value);
            } else {
                self.insert(key, value);
            }
        }
    }
}
