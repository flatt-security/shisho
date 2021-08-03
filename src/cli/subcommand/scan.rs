//! This module defines `scan` subcommand.

use crate::{cli::CommonOpts, language::HCL, query::Query, tree::Tree};
use std::convert::TryFrom;
use structopt::StructOpt;

/// `Opts` defines possible options for the `scan` subcommand.
#[derive(StructOpt, Debug)]
pub struct Opts {
    target_path: String,
}

pub fn run(_common_opts: CommonOpts, _opts: Opts) -> i32 {
    // handle code
    let raw_code = "todo";
    let tree: Tree<HCL> = Tree::try_from(raw_code).expect("failed to load code");

    // handle query
    let raw_query = "todo";
    let query = Query::try_from(raw_query).expect("failed to load query.");

    for matched in tree.matches(&query).to_iter() {
        // matched.
        println!("matched: {}", matched.pattern_index);

        for capture in matched.captures {
            println!("\t- {}: {:?}", capture.index, capture.node);
        }
    }

    todo!("not implemented yet");

    return 0;
}
