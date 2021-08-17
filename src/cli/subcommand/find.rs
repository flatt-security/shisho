//! This module defines `check` subcommand.

use crate::{
    cli::CommonOpts,
    language::{Go, Queryable, HCL},
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

/// Check files under the given path with the given rule sets
#[derive(StructOpt, Debug)]
pub struct FindOpts {
    #[structopt()]
    rule: String,

    #[structopt(parse(from_os_str))]
    target_path: Option<PathBuf>,

    #[structopt(short, long)]
    lang: String,

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
    let lang = ruleset::Language::from_str(opts.lang.as_str())?;
    let rule = Rule {
        id: "inline".into(),
        message: "matched with the given rule".into(),
        language: lang,
        constraints: vec![],
        pattern: opts.rule,
        rewrite: opts.rewrite,
    };

    let target = Target::from(opts.target_path)?;
    if target.is_file() {
        if let Some(target_lang) = target.language() {
            if lang == target_lang {
                find(target, rule, &lang)?;
            }
        }
    } else {
        find(target, rule, &lang)?;
    }

    Ok(())
}

fn find(target: Target, rule: Rule, lang: &ruleset::Language) -> Result<()> {
    match lang {
        ruleset::Language::HCL => find_::<HCL>(target, rule),
        ruleset::Language::Go => find_::<Go>(target, rule),
    }
}

fn find_<T: Queryable + 'static>(target: Target, rule: Rule) -> Result<()> {
    let tree = Tree::<T>::try_from(target.body.as_str()).unwrap();
    let ptree = tree.to_partial();

    let findings = rule.find::<T>(&ptree)?;
    let findings = repeat(&rule).zip(findings).collect();
    print_findings::<T>(&target, findings)?;

    Ok(())
}
