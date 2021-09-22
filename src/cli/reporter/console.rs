use super::Reporter;
use crate::core::{
    code::Code, language::Queryable, matcher::MatchedItem, node::Range, ruleset::Rule,
    target::Target, transform::Transformable,
};
use ansi_term::{Color, Style};
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

            for (idx, rewrite) in rule.get_rewrite_options()?.into_iter().enumerate() {
                if idx > 0 {
                    writeln!(self.writer)?;
                }
                writeln!(self.writer, "Suggested changes ({}):", idx + 1)?;
                let old_code: Code<T> = target.body.clone().into();
                let new_code = old_code.transform(&mitem, rewrite)?;

                let diff = TextDiff::from_lines(target.body.as_str(), new_code.as_str());
                for (group_idx, group) in diff.grouped_ops(1).iter().enumerate() {
                    if group_idx > 0 {
                        writeln!(self.writer)?;
                        writeln!(
                            self.writer,
                            "{:<2} {} {:<2} | {}",
                            "",
                            Style::default().dimmed().paint("..."),
                            "",
                            Style::default().dimmed().paint("...")
                        )?;
                        writeln!(self.writer)?;
                    }
                    for op in group {
                        for change in diff.iter_inline_changes(op) {
                            let (sign, s): (&str, Style) = match change.tag() {
                                ChangeTag::Delete => ("-", Color::Red.into()),
                                ChangeTag::Insert => ("+", Color::Green.into()),
                                ChangeTag::Equal => (" ", Style::default()),
                            };
                            write!(
                                self.writer,
                                "{} {} | {} ",
                                Style::default().dimmed().paint(format!(
                                    "{:<4}",
                                    change
                                        .old_index()
                                        .map(|x| (x + 1).to_string())
                                        .unwrap_or_default()
                                )),
                                Style::default().dimmed().paint(format!(
                                    "{:<4}",
                                    change
                                        .new_index()
                                        .map(|x| (x + 1).to_string())
                                        .unwrap_or_default()
                                )),
                                s.clone().bold().paint(sign),
                            )?;
                            for (emphasized, value) in change.iter_strings_lossy() {
                                if emphasized {
                                    write!(self.writer, "{}", s.clone().underline().paint(value))?;
                                } else {
                                    write!(self.writer, "{}", s.clone().paint(value))?;
                                }
                            }
                            if change.missing_newline() {
                                writeln!(self.writer)?;
                            }
                        }
                    }
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
