//! This module defines `check` subcommand.

use crate::{
    cli::CommonOpts,
    exporter::{Exporter, StdoutExporter},
    language::{Dockerfile, Go, Queryable, HCL},
    ruleset::{self, Rule},
    target::Target,
    tree::Tree,
};
use ansi_term::Color;
use anyhow::Result;
use std::path::PathBuf;
use std::{convert::TryFrom, iter::repeat};
use structopt::StructOpt;

/// Checks files with a pattern given in command line arguments
#[derive(StructOpt, Debug)]
pub struct FindOpts {
    /// Code pattern for searching
    #[structopt()]
    pub pattern: String,

    /// File path to search
    #[structopt(parse(from_os_str))]
    pub target_path: Option<PathBuf>,

    /// Language name to use
    #[structopt(short, long)]
    pub lang: ruleset::Language,

    /// Rewriting pattern
    #[structopt(long)]
    pub rewrite: Option<String>,
}

pub fn run(common_opts: CommonOpts, opts: FindOpts) -> i32 {
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

fn run_(_common_opts: CommonOpts, opts: FindOpts) -> Result<usize> {
    let rule = Rule {
        id: "inline".into(),
        message: "matched with the given rule".into(),
        language: opts.lang,
        constraints: vec![],
        pattern: opts.pattern,
        rewrite: opts.rewrite,
    };

    let mut total_findings = 0;
    match opts.target_path {
        Some(p) if p.is_dir() => {
            for target in Target::iter_from(p) {
                if let Some(target_lang) = target.language() {
                    if opts.lang == target_lang {
                        total_findings += run_rule(target, &rule, &opts.lang)?;
                    }
                }
            }
        }
        Some(p) => {
            let target = Target::from(Some(p))?;
            if let Some(target_lang) = target.language() {
                if opts.lang == target_lang {
                    total_findings += run_rule(target, &rule, &opts.lang)?;
                }
            }
        }
        _ => {
            let target = Target::from(None)?;
            total_findings += run_rule(target, &rule, &opts.lang)?;
        }
    }

    Ok(total_findings)
}

fn run_rule(target: Target, rule: &Rule, lang: &ruleset::Language) -> Result<usize> {
    match lang {
        ruleset::Language::HCL => find_::<HCL>(target, rule),
        ruleset::Language::Dockerfile => find_::<Dockerfile>(target, rule),
        ruleset::Language::Go => find_::<Go>(target, rule),
    }
}

fn find_<T: Queryable + 'static>(target: Target, rule: &Rule) -> Result<usize> {
    let tree = Tree::<T>::try_from(target.body.as_str()).unwrap();
    let ptree = tree.to_partial();

    // TODO
    let exporter = StdoutExporter {};

    let findings = rule.find::<T>(&ptree)?;
    let length = findings.len();
    exporter.run::<T>(&target, repeat(rule).zip(findings).collect())?;

    Ok(length)
}
