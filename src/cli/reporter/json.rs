use std::convert::TryFrom;

use super::Reporter;
use crate::core::{
    language::Queryable,
    matcher::MatchedItem,
    node::{Node, Range},
    ruleset::{filter::PatternWithFilters, Rule},
    source::Code,
    target::Target,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use similar::TextDiff;

pub struct JSONReporter<'a, Writer: std::io::Write> {
    writer: &'a mut Writer,
    entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    pub id: String,
    pub location: Location,
    pub rewrite: Vec<JSONPatch>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Location {
    file: String,
    range: Range,
}

#[derive(Debug, Serialize, Deserialize)]
struct JSONPatch {
    pub diff: String,
}

impl<'a, W: std::io::Write> Reporter<'a> for JSONReporter<'a, W> {
    type Writer = W;
    fn new(writer: &'a mut Self::Writer) -> Self {
        Self {
            writer,
            entries: vec![],
        }
    }

    fn add_entry<'tree, T: Queryable>(
        &mut self,
        target: &Target,
        items: Vec<(&Rule, MatchedItem<'tree, Node<'tree>>)>,
    ) -> Result<()> {
        for (rule, mitem) in items {
            let mut r = Entry {
                id: rule.id.clone(),
                location: Location {
                    file: target.relative_path(),
                    range: mitem.area.range::<T>(),
                },
                rewrite: vec![],
            };

            for rewrite in rule.get_rewrite_options()? {
                let old_code: Code<T> = target.body.clone().into();
                let pattern = PatternWithFilters::try_from(rewrite)?;
                let new_code = old_code.rewrite(&mitem, pattern.as_roption())?;

                let diff = TextDiff::from_lines(target.body.as_str(), new_code.as_str())
                    .unified_diff()
                    .to_string();
                r.rewrite.push(JSONPatch { diff });
            }

            self.entries.push(r);
        }

        Ok(())
    }

    fn report(&mut self) -> Result<()> {
        let s = serde_json::to_string(&self.entries)?;
        self.entries = vec![];
        write!(self.writer, "{}", s)?;
        Ok(())
    }
}
