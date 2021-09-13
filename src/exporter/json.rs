use super::Exporter;
use crate::code::Code;
use crate::transform::Transformable;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use similar::TextDiff;

pub struct JSONExporter<'a, Writer: std::io::Write> {
    writer: &'a mut Writer,
    buffer: Vec<JSONResult>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JSONResult {
    pub id: String,
    pub file: String,
    pub rewrite: Vec<JSONPatch>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JSONPatch {
    pub diff: String,
}

impl<'a, W: std::io::Write> Exporter<'a> for JSONExporter<'a, W> {
    type Writer = W;
    fn new(writer: &'a mut Self::Writer) -> Self {
        Self {
            writer,
            buffer: vec![],
        }
    }

    fn run<T: crate::language::Queryable + 'static>(
        &mut self,
        target: &crate::target::Target,
        items: Vec<(&crate::ruleset::Rule, crate::matcher::MatchedItem)>,
    ) -> Result<()> {
        let target_path = if let Some(ref p) = target.path {
            let p = p.canonicalize()?;
            p.to_string_lossy().to_string()
        } else {
            "/dev/stdin".to_string()
        };

        for (rule, mitem) in items {
            let mut r = JSONResult {
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
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        let s = serde_json::to_string(&self.buffer)?;
        self.buffer = vec![];
        write!(self.writer, "{}", s)?;
        Ok(())
    }
}
