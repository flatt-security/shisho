use super::node::{NodeLike, NodeLikeId};

pub trait NodeLikeView<'tree, N: NodeLike<'tree>> {
    fn root(&'tree self) -> Option<&'tree N>;
    fn get(&'tree self, id: NodeLikeId<'tree, N>) -> Option<&'tree N>;
}
