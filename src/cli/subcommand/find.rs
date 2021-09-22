//! This module defines `check` subcommand.

use crate::cli::encoding::{parse_encoding, LABELS_SORTED};
use crate::cli::reporter::{ConsoleReporter, JSONReporter, Reporter, ReporterType, SARIFReporter};
use crate::cli::{subcommand::check::handle_rulemap, CommonOpts, ReportOpts};
use crate::core::ruleset::{self, Rule};
use ansi_term::Color;
use anyhow::Result;
use encoding_rs::Encoding;
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

    #[structopt(flatten)]
    pub common: CommonOpts,

    #[structopt(short, long, parse(try_from_str = parse_encoding), possible_values(&LABELS_SORTED))]
    pub encoding: Option<&'static Encoding>,

    #[structopt(flatten)]
    pub report: ReportOpts,
}

pub fn run(opts: FindOpts) -> i32 {
    match handle_opts(opts) {
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

fn handle_opts(opts: FindOpts) -> Result<usize> {
    let rule = Rule {
        id: "inline".into(),
        message: "matched with the given rule".into(),
        language: opts.lang,
        constraints: vec![],
        pattern: opts.pattern,
        tags: vec![],
        rewrite: opts.rewrite,
    };

    let rule_map =
        IntoIter::new([(opts.lang, vec![rule])]).collect::<HashMap<ruleset::Language, Vec<Rule>>>();

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    match opts.report.format {
        ReporterType::JSON => handle_rulemap(
            JSONReporter::new(&mut stdout),
            opts.target_path,
            opts.encoding,
            rule_map,
        ),
        ReporterType::Console => handle_rulemap(
            ConsoleReporter::new(&mut stdout),
            opts.target_path,
            opts.encoding,
            rule_map,
        ),
        ReporterType::SARIF => handle_rulemap(
            SARIFReporter::new(&mut stdout),
            opts.target_path,
            opts.encoding,
            rule_map,
        ),
    }
}
