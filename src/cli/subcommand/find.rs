//! This module defines `check` subcommand.

use crate::{
    cli::CommonOpts,
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

use super::check::print_findings;

/// Checks files with a pattern given in command line arguments
#[derive(StructOpt, Debug)]
pub struct FindOpts {
    /// Code pattern for searching
    #[structopt()]
    pattern: String,

    /// File path to search
    #[structopt(parse(from_os_str))]
    target_path: Option<PathBuf>,

    /// Language name to use
    #[structopt(short, long)]
    lang: ruleset::Language,

    /// Rewriting pattern
    #[structopt(long)]
    rewrite: Option<String>,
}

pub fn run(common_opts: CommonOpts, opts: FindOpts) -> i32 {
    match run_(common_opts, opts) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{}: {}", Color::Red.paint("error"), e);
            1
        }
    }
}

fn run_(_common_opts: CommonOpts, opts: FindOpts) -> Result<()> {
    let rule = Rule {
        id: "inline".into(),
        message: "matched with the given rule".into(),
        language: opts.lang,
        constraints: vec![],
        pattern: opts.pattern,
        rewrite: opts.rewrite,
    };

    match opts.target_path {
        Some(p) if p.is_dir() => {
            for target in Target::iter_from(p) {
                if let Some(target_lang) = target.language() {
                    if opts.lang == target_lang {
                        run_rule(target, &rule, &opts.lang)?;
                    }
                }
            }
        }
        Some(p) => {
            let target = Target::from(Some(p))?;
            if let Some(target_lang) = target.language() {
                if opts.lang == target_lang {
                    run_rule(target, &rule, &opts.lang)?;
                }
            }
        }
        _ => {
            let target = Target::from(None)?;
            run_rule(target, &rule, &opts.lang)?;
        }
    }

    Ok(())
}

fn run_rule(target: Target, rule: &Rule, lang: &ruleset::Language) -> Result<()> {
    match lang {
        ruleset::Language::HCL => find_::<HCL>(target, rule),
        ruleset::Language::Dockerfile => find_::<Dockerfile>(target, rule),
        ruleset::Language::Go => find_::<Go>(target, rule),
    }
}

fn find_<T: Queryable + 'static>(target: Target, rule: &Rule) -> Result<()> {
    let tree = Tree::<T>::try_from(target.body.as_str()).unwrap();
    let ptree = tree.to_partial();

    let findings = rule.find::<T>(&ptree)?;
    let findings = repeat(rule).zip(findings).collect();
    print_findings::<T>(&target, findings)?;

    Ok(())
}
