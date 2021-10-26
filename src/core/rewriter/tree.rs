use super::node::RewritableNode;
use crate::core::{language::Queryable, source::NormalizedSource};
use std::marker::PhantomData;

pub struct NormalizedRewritableTree<'tree, T> {
    pub root: RewritableNode,
    pub source: &'tree NormalizedSource,
    _marker: PhantomData<T>,
}

impl<'tree, T> NormalizedRewritableTree<'tree, T>
where
    T: Queryable,
{
    pub fn new(view_root: RewritableNode, source: &'tree NormalizedSource) -> Self {
        Self {
            root: view_root,
            source,
            _marker: PhantomData,
        }
    }
}
