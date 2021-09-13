//! This module defines `check` subcommand.

use crate::{
    cli::CommonOpts,
    exporter::{ConsoleExporter, Exporter},
    language::{Dockerfile, Go, Queryable, HCL},
    ruleset::{self, Rule},
    target::Target,
    tree::Tree,
};
use ansi_term::Color;
use anyhow::{anyhow, Result};
use std::{collections::HashMap, convert::TryFrom};
use std::{iter::repeat, path::PathBuf};
use structopt::StructOpt;

/// Checks files under the given path with the given rule sets
#[derive(StructOpt, Debug)]
pub struct CheckOpts {
    /// Rule Set for searching
    #[structopt(parse(from_os_str))]
    pub ruleset_path: PathBuf,

    /// File path to search    
    #[structopt(parse(from_os_str))]
    pub target_path: Option<PathBuf>,
}

pub fn run(common_opts: CommonOpts, opts: CheckOpts) -> i32 {
    match run_(common_opts, opts) {
        Ok(total_findings) => {
            if total_findings > 0 {
                1
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("{}: {}", Color::Red.paint("error"), e);
            1
        }
    }
}

fn run_(_common_opts: CommonOpts, opts: CheckOpts) -> Result<usize> {
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
    let mut total_findings = 0;
    match opts.target_path {
        Some(p) if p.is_dir() => {
            for target in Target::iter_from(p) {
                if let Some(lang) = target.language() {
                    if let Some(rules) = rule_map.get(&lang) {
                        total_findings += run_rules(&target, rules, &lang)?;
                    }
                }
            }
        }
        Some(p) => {
            let target = Target::from(Some(p))?;
            if let Some(lang) = target.language() {
                if let Some(rules) = rule_map.get(&lang) {
                    total_findings += run_rules(&target, rules, &lang)?;
                }
            }
        }
        _ => {
            let target = Target::from(None)?;
            for (lang, rules) in rule_map {
                total_findings += run_rules(&target, &rules, &lang)?;
            }
        }
    }

    Ok(total_findings)
}

fn run_rules(target: &Target, rules: &Vec<Rule>, lang: &ruleset::Language) -> Result<usize> {
    match lang {
        ruleset::Language::HCL => run_rules_::<HCL>(&target, rules),
        ruleset::Language::Dockerfile => run_rules_::<Dockerfile>(&target, rules),
        ruleset::Language::Go => run_rules_::<Go>(&target, rules),
    }
}

fn run_rules_<T: Queryable + 'static>(target: &Target, rules: &Vec<Rule>) -> Result<usize> {
    let tree = Tree::<T>::try_from(target.body.as_str()).unwrap();
    let ptree = tree.to_partial();

    // TODO
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let mut exporter = ConsoleExporter::new(&mut stdout);

    let mut total_findings = 0;
    for rule in rules {
        let findings = rule.find::<T>(&ptree)?;
        total_findings += findings.len();
        exporter.run::<T>(target, repeat(rule).zip(findings).collect())?;
    }

    Ok(total_findings)
}
