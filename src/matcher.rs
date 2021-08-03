use crate::{language::Queryable, query::Query, tree::Tree};

pub struct QueryMatcher<'tree, 'query, T>
where
    T: Queryable,
    'tree: 'query,
{
    cursor: tree_sitter::QueryCursor,
    query: &'query Query<T>,
    tree: &'tree Tree<'tree, T>,
}

impl<'tree, 'query, T> QueryMatcher<'tree, 'query, T>
where
    T: Queryable,
    'tree: 'query,
{
    pub fn new(tree: &'tree Tree<'tree, T>, query: &'query Query<T>) -> Self {
        let cursor = tree_sitter::QueryCursor::new();
        QueryMatcher {
            tree,
            cursor,
            query,
        }
    }

    pub fn as_iter(&'query mut self) -> impl Iterator<Item = MatchedItem<'query>> + 'query {
        let raw = self.tree.raw;
        self.cursor.matches(
            self.query.ts_query(),
            self.tree.ts_tree().root_node(),
            move |x| x.utf8_text(raw).unwrap(),
        )
    }
}

pub type MatchedItem<'a> = tree_sitter::QueryMatch<'a>;
