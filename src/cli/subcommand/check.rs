//! This module defines `check` subcommand.

use crate::cli::encoding::{parse_encoding, LABELS_SORTED};
use crate::cli::reporter::{ConsoleReporter, JSONReporter, Reporter, ReporterType, SARIFReporter};
use crate::cli::{CommonOpts, ReportOpts};
use crate::core::source::NormalizedSource;
use crate::core::target::TargetLoader;
use crate::core::tree::NormalizedTree;
use crate::core::{
    language::{Dockerfile, Go, Queryable, HCL},
    ruleset::{self, Rule},
    target::Target,
    tree::Tree,
};
use ansi_term::Color;
use anyhow::{anyhow, Result};
use encoding_rs::Encoding;
use std::{collections::HashMap, convert::TryFrom};
use std::{iter::repeat, path::PathBuf};
use structopt::StructOpt;

// Checks files under the given path with the given rule sets
#[derive(StructOpt, Debug)]
pub struct CheckOpts {
    /// Rule Set for searching
    #[structopt(parse(from_os_str))]
    pub ruleset_path: PathBuf,

    /// File path to search    
    #[structopt(parse(from_os_str))]
    pub target_path: Option<PathBuf>,

    #[structopt(flatten)]
    pub common: CommonOpts,

    #[structopt(short, long, parse(try_from_str = parse_encoding), possible_values(&LABELS_SORTED))]
    pub encoding: Option<&'static Encoding>,

    #[structopt(long)]
    pub exit_zero: bool,

    #[structopt(flatten)]
    pub report: ReportOpts,

    #[structopt(long)]
    pub exclude: Vec<String>,
}

pub fn run(opts: CheckOpts) -> i32 {
    let exit_zero = opts.exit_zero;
    match handle_opts(opts) {
        Ok(total_findings) => {
            if total_findings > 0 && !exit_zero {
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

pub(crate) fn handle_opts(opts: CheckOpts) -> Result<usize> {
    let mut rule_map = HashMap::<ruleset::Language, Vec<Rule>>::new();

    let rulesets = ruleset::from_path(&opts.ruleset_path).map_err(|e| {
        anyhow!(
            "failed to load ruleset file {}: {}",
            opts.ruleset_path.as_os_str().to_string_lossy(),
            e
        )
    })?;

    for ruleset in rulesets {
        for rule in ruleset.rules {
            if let Some(v) = rule_map.get_mut(&rule.language) {
                v.push(rule);
            } else {
                rule_map.insert(rule.language, vec![rule]);
            }
        }
    }

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    match opts.report.format {
        ReporterType::JSON => handle_rulemap(
            JSONReporter::new(&mut stdout),
            opts.target_path,
            opts.exclude,
            opts.encoding,
            rule_map,
        ),
        ReporterType::Console => handle_rulemap(
            ConsoleReporter::new(&mut stdout),
            opts.target_path,
            opts.exclude,
            opts.encoding,
            rule_map,
        ),
        ReporterType::SARIF => handle_rulemap(
            SARIFReporter::new(&mut stdout),
            opts.target_path,
            opts.exclude,
            opts.encoding,
            rule_map,
        ),
    }
}

pub(crate) fn handle_rulemap<'a>(
    mut reporter: impl Reporter<'a>,
    target_path: Option<PathBuf>,

    exclude_path_pattern: Vec<String>,
    encoding: Option<&'static Encoding>,
    rule_map: HashMap<ruleset::Language, Vec<Rule>>,
) -> Result<usize> {
    let mut total_findings = 0;
    let loader = TargetLoader::new(exclude_path_pattern, encoding)?;
    match target_path {
        Some(p) => {
            for target in loader.from(p)? {
                if let Some(lang) = target.language() {
                    if let Some(rules) = rule_map.get(&lang) {
                        total_findings += handle_rules(&mut reporter, &target, rules, &lang)?;
                    }
                }
            }
        }
        _ => {
            let target = loader.from_reader(std::io::stdin())?;
            for (lang, rules) in rule_map {
                total_findings += handle_rules(&mut reporter, &target, &rules, &lang)?;
            }
        }
    }

    reporter.report()?;
    Ok(total_findings)
}

fn handle_rules<'a, E: Reporter<'a>>(
    reporter: &mut E,
    target: &Target,
    rules: &[Rule],
    as_lang: &ruleset::Language,
) -> Result<usize> {
    match as_lang {
        ruleset::Language::HCL => handle_typed_rules::<E, HCL>(reporter, target, rules),
        ruleset::Language::Dockerfile => {
            handle_typed_rules::<E, Dockerfile>(reporter, target, rules)
        }
        ruleset::Language::Go => handle_typed_rules::<E, Go>(reporter, target, rules),
    }
}

fn handle_typed_rules<'a, E: Reporter<'a>, Lang: Queryable>(
    reporter: &mut E,
    target: &Target,
    rules: &[Rule],
) -> Result<usize> {
    let source = NormalizedSource::from(target.body.as_str());
    let tree = Tree::<Lang>::try_from(source).unwrap();
    let ptree = NormalizedTree::from(&tree);
    let ptree = ptree.as_ref_treeview();

    let mut total_findings = 0;
    for rule in rules {
        let findings = rule.find::<Lang>(&ptree)?;
        total_findings += findings.len();
        reporter.add_entry::<Lang>(target, repeat(rule).zip(findings).collect())?;
    }

    Ok(total_findings)
}
