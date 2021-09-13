//! This module defines `check` subcommand.

use crate::{
    cli::{subcommand::check::run_with_rulemap, CommonOpts},
    ruleset::{self, Rule},
};
use ansi_term::Color;
use anyhow::Result;
use std::{array::IntoIter, collections::HashMap, path::PathBuf};
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

    let rule_map =
        IntoIter::new([(opts.lang, vec![rule])]).collect::<HashMap<ruleset::Language, Vec<Rule>>>();

    run_with_rulemap(opts.target_path, rule_map)
}
