pub trait Merge {
    fn merge(&mut self, other: Self);
}

pub trait Concat {
    fn concat(&mut self, other: &Self);
}
