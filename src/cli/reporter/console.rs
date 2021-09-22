use super::Reporter;
use crate::core::{
    code::Code, language::Queryable, matcher::MatchedItem, node::Range, ruleset::Rule,
    target::Target, transform::Transformable,
};
use ansi_term::Color;
use anyhow::Result;
use similar::{ChangeTag, TextDiff};

pub struct ConsoleReporter<'a, Writer: std::io::Write> {
    writer: &'a mut Writer,
}

impl<'a, W: std::io::Write> Reporter<'a> for ConsoleReporter<'a, W> {
    type Writer = W;
    fn new(writer: &'a mut Self::Writer) -> Self {
        Self { writer }
    }

    fn add_entry<T: Queryable + 'static>(
        &mut self,
        target: &Target,
        items: Vec<(&Rule, MatchedItem)>,
    ) -> Result<()> {
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
            writeln!(self.writer, "In {}:", target.canonicalized_path())?;
            writeln!(self.writer, "{:>8} |", "")?;
            let Range { start: s, end: e } = mitem.area.range::<T>(target.body.as_ref());

            for line_index in (s.row)..=(e.row) {
                if line_index > lines.len() || (line_index == e.row && e.column == 0) {
                    continue;
                }

                let line_value = lines[line_index - 1]; // since `Range.row` is 1-indexed

                let v = match line_index {
                    line if line == s.row && line == e.row => vec![
                        line_value[..(s.column - 1)].to_string(),
                        Color::Yellow
                            .paint(&line_value[(s.column - 1)..(e.column - 1)])
                            .to_string(),
                        line_value[(e.column - 1)..line_value.len()].to_string(),
                    ],
                    line if line == s.row => vec![
                        line_value[..(s.column - 1)].to_string(),
                        Color::Yellow
                            .paint(&line_value[(s.column - 1)..line_value.len()])
                            .to_string(),
                    ],

                    line if line == e.row => vec![
                        Color::Yellow
                            .paint(&line_value[..(e.column - 1)])
                            .to_string(),
                        line_value[(e.column - 1)..line_value.len()].to_string(),
                    ],

                    _ => vec![Color::Yellow.paint(line_value).to_string()],
                };

                writeln!(
                    self.writer,
                    "{} | {}",
                    Color::Green.paint(format!("{:>8}", line_index.to_string())),
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

    fn report(&mut self) -> Result<()> {
        Ok(())
    }
}
