mod console;
use std::str::FromStr;

pub use self::console::*;

mod json;
pub use self::json::*;

use crate::core::{language::Queryable, matcher::MatchedItem, ruleset::Rule, target::Target};
use anyhow::Result;

pub trait Reporter<'a> {
    type Writer: std::io::Write;
    fn new(writer: &'a mut Self::Writer) -> Self
    where
        Self: Sized;

    fn add_entry<T: Queryable + 'static>(
        &mut self,
        target: &Target,
        items: Vec<(&Rule, MatchedItem)>,
    ) -> Result<()>;

    fn report(&mut self) -> Result<()>;
}

#[derive(Debug)]
pub enum ReporterType {
    JSON,
    Console,
}

impl FromStr for ReporterType {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "json" => Ok(ReporterType::JSON),
            "console" => Ok(ReporterType::Console),
            _ => Err("".into()),
        }
    }
}

impl ReporterType {
    pub fn variants() -> [&'static str; 2] {
        ["json", "console"]
    }
}
