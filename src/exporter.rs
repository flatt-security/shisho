mod console;
pub use self::console::*;

use crate::{language::Queryable, matcher::MatchedItem, ruleset::Rule, target::Target};
use anyhow::Result;

pub trait Exporter<'a> {
    type Writer: std::io::Write;
    fn new(writer: &'a mut Self::Writer) -> Self;
    fn run<T: Queryable + 'static>(
        &mut self,
        target: &Target,
        items: Vec<(&Rule, MatchedItem)>,
    ) -> Result<()>;
}
