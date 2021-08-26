use crate::language::Queryable;

#[derive(Debug, Clone, PartialEq)]
pub struct ConsecutiveNodes<'tree>(Vec<tree_sitter::Node<'tree>>);

impl<'tree> From<Vec<tree_sitter::Node<'tree>>> for ConsecutiveNodes<'tree> {
    fn from(value: Vec<tree_sitter::Node<'tree>>) -> Self {
        if value.len() == 0 {
            panic!("internal error; ConsecutiveNodes was generated from empty vec.");
        }
        ConsecutiveNodes(value)
    }
}

impl<'tree> From<Vec<ConsecutiveNodes<'tree>>> for ConsecutiveNodes<'tree> {
    fn from(cns: Vec<ConsecutiveNodes<'tree>>) -> Self {
        ConsecutiveNodes(cns.into_iter().map(|cn| cn.0).flatten().collect())
    }
}

impl<'tree> ConsecutiveNodes<'tree> {
    pub fn as_vec(&self) -> &Vec<tree_sitter::Node<'tree>> {
        &self.0
    }

    pub fn push(&mut self, n: tree_sitter::Node<'tree>) {
        self.0.push(n)
    }

    pub fn start_position(&self) -> tree_sitter::Point {
        self.as_vec().first().unwrap().start_position()
    }

    pub fn end_position(&self) -> tree_sitter::Point {
        self.as_vec().last().unwrap().end_position()
    }

    pub fn range_for_view<T: Queryable + 'static>(
        &self,
    ) -> (tree_sitter::Point, tree_sitter::Point) {
        (
            T::range_for_view(self.as_vec().first().unwrap()).0,
            T::range_for_view(self.as_vec().last().unwrap()).1,
        )
    }

    pub fn start_byte(&self) -> usize {
        self.as_vec().first().unwrap().start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.as_vec().last().unwrap().end_byte()
    }

    pub fn utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, core::str::Utf8Error> {
        core::str::from_utf8(&source[self.start_byte()..self.end_byte()])
    }
}
