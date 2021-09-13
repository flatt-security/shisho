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
    match run_with_opts(common_opts, opts) {
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

fn run_with_opts(_common_opts: CommonOpts, opts: CheckOpts) -> Result<usize> {
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

    run_with_rulemap(opts.target_path, rule_map)
}

pub(crate) fn run_with_rulemap(
    target_path: Option<PathBuf>,
    rule_map: HashMap<ruleset::Language, Vec<Rule>>,
) -> Result<usize> {
    // TODO: prepare exporter
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let mut exporter = ConsoleExporter::new(&mut stdout);

    // run rules
    let mut total_findings = 0;
    match target_path {
        Some(p) if p.is_dir() => {
            for target in Target::iter_from(p) {
                if let Some(lang) = target.language() {
                    if let Some(rules) = rule_map.get(&lang) {
                        total_findings += handle_rules(&mut exporter, &target, rules, &lang)?;
                    }
                }
            }
        }
        Some(p) => {
            let target = Target::from(Some(p))?;
            if let Some(lang) = target.language() {
                if let Some(rules) = rule_map.get(&lang) {
                    total_findings += handle_rules(&mut exporter, &target, rules, &lang)?;
                }
            }
        }
        _ => {
            let target = Target::from(None)?;
            for (lang, rules) in rule_map {
                total_findings += handle_rules(&mut exporter, &target, &rules, &lang)?;
            }
        }
    }

    exporter.flush()?;
    Ok(total_findings)
}

fn handle_rules<'a, E: Exporter<'a>>(
    exporter: &mut E,
    target: &Target,
    rules: &Vec<Rule>,
    as_lang: &ruleset::Language,
) -> Result<usize> {
    match as_lang {
        ruleset::Language::HCL => handle_typed_rules::<E, HCL>(exporter, &target, rules),
        ruleset::Language::Dockerfile => {
            handle_typed_rules::<E, Dockerfile>(exporter, &target, rules)
        }
        ruleset::Language::Go => handle_typed_rules::<E, Go>(exporter, &target, rules),
    }
}

fn handle_typed_rules<'a, E: Exporter<'a>, Lang: Queryable + 'static>(
    exporter: &mut E,
    target: &Target,
    rules: &Vec<Rule>,
) -> Result<usize> {
    let tree = Tree::<Lang>::try_from(target.body.as_str()).unwrap();
    let ptree = tree.to_partial();

    let mut total_findings = 0;
    for rule in rules {
        let findings = rule.find::<Lang>(&ptree)?;
        total_findings += findings.len();
        exporter.run::<Lang>(target, repeat(rule).zip(findings).collect())?;
    }

    Ok(total_findings)
}
