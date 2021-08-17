//! This module defines `scan` subcommand.

use crate::{
    cli::CommonOpts,
    language::{Go, Queryable, HCL},
    ruleset::{self, Rule},
    target::Target,
    tree::{PartialTree, Tree},
};
use ansi_term::Color;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::{collections::HashMap, convert::TryFrom};
use structopt::StructOpt;

/// Scans files under the given path with the given rule sets
#[derive(StructOpt, Debug)]
pub struct ScanOpts {
    #[structopt(parse(from_os_str))]
    target_path: PathBuf,

    #[structopt(required_unless_all(&["lang", "rule"]))]
    ruleset_path: Option<PathBuf>,

    #[structopt(short, long)]
    lang: Option<String>,
    #[structopt(short, long)]
    rule: Option<String>,
}

pub fn run(common_opts: CommonOpts, opts: ScanOpts) -> i32 {
    match intl(common_opts, opts) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn intl(_common_opts: CommonOpts, opts: ScanOpts) -> Result<()> {
    let mut rule_map = HashMap::<ruleset::Language, Vec<Rule>>::new();
    match (opts.ruleset_path, opts.lang, opts.rule) {
        (Some(ruleset_path), _, _) => {
            let ruleset = ruleset::from_reader(&ruleset_path).map_err(|e| {
                anyhow!(
                    "failed to load ruleset file {}: {}",
                    ruleset_path.as_os_str().to_string_lossy(),
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
        }
        (None, Some(lang), Some(rule_raw)) => {
            let lang: ruleset::Language = serde_yaml::from_str(lang.as_str())?;
            let rule = Rule {
                id: "inline".into(),
                language: lang,
                message: "".into(),
                pattern: rule_raw,
                constraints: vec![],
            };
            rule_map.insert(lang, vec![rule]);
        }
        _ => {
            panic!("todo");
        }
    };

    let target = Target::new(opts.target_path)?;

    if let Some(lang) = target.language() {
        if let Some(rules) = rule_map.get(&lang) {
            match lang {
                ruleset::Language::HCL => {
                    run_rules::<HCL>(rules, &target)?;
                }
                ruleset::Language::Go => {
                    run_rules::<Go>(rules, &target)?;
                }
            };
        }
    }

    Ok(())
}

fn run_rules<T: Queryable + 'static>(rules: &Vec<Rule>, target: &Target) -> Result<()> {
    let tree = Tree::<T>::try_from(target.body.as_str()).unwrap();
    let ptree = tree.to_partial();

    for rule in rules {
        run_rule(rule, target, &ptree)?;
    }

    Ok(())
}

fn run_rule<T: Queryable + 'static>(
    rule: &Rule,
    target: &Target,
    ptree: &PartialTree<T>,
) -> Result<()> {
    let target_path = target.path.canonicalize()?;
    let target_path = target_path
        .to_str()
        .ok_or(anyhow!("failed to interpret the path"))?;
    let lines = target.body.split("\n").collect::<Vec<&str>>();

    println!("--------------");
    println!("{}", Color::Green.paint(target_path));

    for mitem in rule.find::<T>(&ptree)? {
        println!("...");

        let s = mitem.top.start_position();
        let e = mitem.top.end_position();
        for line_index in (s.row)..=(e.row) {
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
                Color::Green.paint(line_index.to_string()),
                v.join("")
            );
        }
    }

    Ok(())
}
