//! This module defines `check` subcommand.

use crate::{
    cli::CommonOpts,
    code::Code,
    language::{Go, Queryable, HCL},
    matcher::MatchedItem,
    ruleset::{self, Rule},
    target::Target,
    transform::Transformable,
    tree::Tree,
};
use ansi_term::Color;
use anyhow::{anyhow, Result};
use similar::{ChangeTag, TextDiff};
use std::{collections::HashMap, convert::TryFrom};
use std::{iter::repeat, path::PathBuf};
use structopt::StructOpt;

/// Checks files under the given path with the given rule sets
#[derive(StructOpt, Debug)]
pub struct CheckOpts {
    /// Rule Set for searching
    #[structopt(parse(from_os_str))]
    ruleset_path: PathBuf,

    /// File path to search    
    #[structopt(parse(from_os_str))]
    target_path: Option<PathBuf>,
}

pub fn run(common_opts: CommonOpts, opts: CheckOpts) -> i32 {
    match run_(common_opts, opts) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{}: {}", Color::Red.paint("error"), e);
            1
        }
    }
}

fn run_(_common_opts: CommonOpts, opts: CheckOpts) -> Result<()> {
    // load rules
    let mut rule_map = HashMap::<ruleset::Language, Vec<Rule>>::new();
    let ruleset = ruleset::from_reader(&opts.ruleset_path).map_err(|e| {
        anyhow!(
            "failed to load ruleset file {}: {}",
            opts.ruleset_path.as_os_str().to_string_lossy(),
            e
        )
    })?;
    for rule in ruleset.rules {
        if let Some(v) = rule_map.get_mut(&rule.language) {
            v.push(rule);
        } else {
            rule_map.insert(rule.language, vec![rule]);
        }
    }

    // run rules
    match opts.target_path {
        Some(p) if p.is_dir() => {
            for target in Target::iter_from(p) {
                if let Some(lang) = target.language() {
                    if let Some(rules) = rule_map.get(&lang) {
                        run_rules(&target, rules, &lang)?;
                    }
                }
            }
        }
        Some(p) => {
            let target = Target::from(Some(p))?;
            if let Some(lang) = target.language() {
                if let Some(rules) = rule_map.get(&lang) {
                    run_rules(&target, rules, &lang)?;
                }
            }
        }
        _ => {
            let target = Target::from(None)?;
            for (lang, rules) in rule_map {
                run_rules(&target, &rules, &lang)?;
            }
        }
    }

    Ok(())
}

fn run_rules(target: &Target, rules: &Vec<Rule>, lang: &ruleset::Language) -> Result<()> {
    match lang {
        ruleset::Language::HCL => run_rules_::<HCL>(&target, rules),
        ruleset::Language::Go => run_rules_::<Go>(&target, rules),
    }
}

fn run_rules_<T: Queryable + 'static>(target: &Target, rules: &Vec<Rule>) -> Result<()> {
    let tree = Tree::<T>::try_from(target.body.as_str()).unwrap();
    let ptree = tree.to_partial();

    for rule in rules {
        let findings = rule.find::<T>(&ptree)?;
        let findings = repeat(rule).zip(findings).collect();
        print_findings::<T>(target, findings)?;
    }

    Ok(())
}

pub(crate) fn print_findings<T: Queryable + 'static>(
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
        println!(
            "{}: {}",
            Color::Red.paint(format!("[{}]", rule.id)),
            Color::White.bold().paint(rule.message.clone().trim_end())
        );

        // print a finding
        println!("In {}:", target_path);
        println!("{:>8} |", "");
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

            println!(
                "{} | {}",
                Color::Green.paint(format!("{:>8}", (line_index + 1).to_string())),
                v.join("")
            );
        }
        println!("{:>8} |", "");

        // print suggested changes
        if let Some(ref rewrite_pattern) = rule.rewrite {
            println!("Suggested changes:");
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
        println!("");
    }

    Ok(())
}
