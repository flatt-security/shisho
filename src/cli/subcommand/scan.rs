//! This module defines `scan` subcommand.

use std::path::PathBuf;

use crate::{cli::CommonOpts, language::Go, matcher::MatchedItem, query::RawQuery, tree::RawTree};
use structopt::StructOpt;

/// `Opts` defines possible options for the `scan` subcommand.
#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(parse(from_os_str))]
    target_path: PathBuf,

    policy_path: String,
}

pub fn run(_common_opts: CommonOpts, opts: Opts) -> i32 {
    // TODO (y0n3uchy): this is just a sample implementation! we need to improve this
    let file = std::fs::read_to_string(&opts.target_path).unwrap();
    let tree = RawTree::<Go>::new(file.as_str()).into_tree().unwrap();

    let pattern = std::fs::read_to_string(&opts.policy_path)
        .expect(format!("failed to load file: {}", opts.policy_path).as_str());
    let query = RawQuery::<Go>::new(pattern.as_str()).to_query().unwrap();

    let mut session = tree.matches(&query);
    println!(
        "{} items matched",
        session.as_iter().collect::<Vec<MatchedItem>>().len()
    );

    0
}
