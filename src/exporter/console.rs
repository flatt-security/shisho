use super::Exporter;
use crate::code::Code;
use crate::transform::Transformable;
use ansi_term::Color;
use anyhow::Result;
use similar::{ChangeTag, TextDiff};

pub struct ConsoleExporter<'a, Writer: std::io::Write> {
    writer: &'a mut Writer,
}

impl<'a, W: std::io::Write> Exporter<'a> for ConsoleExporter<'a, W> {
    type Writer = W;
    fn new(writer: &'a mut Self::Writer) -> Self {
        Self { writer }
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
        let lines = target.body.split("\n").collect::<Vec<&str>>();

        for (rule, mitem) in items {
            // print metadata of the matched items
            writeln!(
                self.writer,
                "{}: {}",
                Color::Red.paint(format!("[{}]", rule.id)),
                Color::White.bold().paint(rule.message.clone().trim_end())
            )?;

            // print a finding
            writeln!(self.writer, "In {}:", target_path)?;
            writeln!(self.writer, "{:>8} |", "")?;
            let (s, e) = mitem.area.range_for_view::<T>();

            for line_index in (s.row)..=(e.row) {
                if line_index >= lines.len() {
                    continue;
                }
                let line_value = lines[line_index];

                let v = match line_index {
                    line if line == s.row && line == e.row => vec![
                        line_value[..s.column].to_string(),
                        Color::Yellow
                            .paint(&line_value[s.column..e.column])
                            .to_string(),
                        line_value[e.column..line_value.len()].to_string(),
                    ],
                    line if line == s.row => vec![
                        line_value[..s.column].to_string(),
                        Color::Yellow
                            .paint(&line_value[s.column..line_value.len()])
                            .to_string(),
                    ],

                    line if line == e.row => vec![
                        Color::Yellow.paint(&line_value[..e.column]).to_string(),
                        line_value[e.column..line_value.len()].to_string(),
                    ],

                    _ => vec![Color::Yellow.paint(line_value).to_string()],
                };

                writeln!(
                    self.writer,
                    "{} | {}",
                    Color::Green.paint(format!("{:>8}", (line_index + 1).to_string())),
                    v.join("")
                )?;
            }
            writeln!(self.writer, "{:>8} |", "")?;

            // print suggested changes
            if let Some(ref rewrite_pattern) = rule.rewrite {
                writeln!(self.writer, "Suggested changes:")?;
                let old_code: Code<T> = target.body.clone().into();
                let new_code = old_code.transform(mitem, rewrite_pattern.as_str())?;

                let diff = TextDiff::from_lines(target.body.as_str(), new_code.as_str());
                for change in diff.iter_all_changes() {
                    match change.tag() {
                        ChangeTag::Delete => print!(
                            "{} | {}",
                            Color::Green.paint(format!("{:>8}", (change.old_index().unwrap() + 1))),
                            Color::Red.paint(format!("-{}", change))
                        ),
                        ChangeTag::Insert => print!(
                            "{} | {}",
                            Color::Green.paint(format!("{:>8}", (change.new_index().unwrap() + 1))),
                            Color::Green.paint(format!("+{}", change))
                        ),
                        ChangeTag::Equal => (),
                    };
                }
            }

            // print a separator between matched items
            writeln!(self.writer, "")?;
        }

        Ok(())
    }
}
