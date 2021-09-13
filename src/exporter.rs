mod console;
pub use self::console::*;

mod json;
pub use self::json::*;

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
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
