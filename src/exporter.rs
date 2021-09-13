mod stdout;
pub use self::stdout::*;

use crate::{language::Queryable, matcher::MatchedItem, ruleset::Rule, target::Target};
use anyhow::Result;

pub trait Exporter {
    fn run<T: Queryable + 'static>(
        &self,
        target: &Target,
        items: Vec<(&Rule, MatchedItem)>,
    ) -> Result<()>;
}
