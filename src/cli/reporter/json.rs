use super::Reporter;
use crate::core::{
    code::Code, language::Queryable, matcher::MatchedItem, ruleset::Rule, target::Target,
    transform::Transformable,
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
    pub file: String,
    pub rewrite: Vec<JSONPatch>,
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

    fn add_entry<T: Queryable + 'static>(
        &mut self,
        target: &Target,
        items: Vec<(&Rule, MatchedItem)>,
    ) -> Result<()> {
        let target_path = if let Some(ref p) = target.path {
            let p = p.canonicalize()?;
            p.to_string_lossy().to_string()
        } else {
            "/dev/stdin".to_string()
        };

        for (rule, mitem) in items {
            let mut r = Entry {
                id: rule.id.clone(),
                file: target_path.clone(),
                rewrite: vec![],
            };

            if let Some(ref rewrite_pattern) = rule.rewrite {
                let old_code: Code<T> = target.body.clone().into();
                let new_code = old_code.transform(mitem, rewrite_pattern.as_str())?;

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
