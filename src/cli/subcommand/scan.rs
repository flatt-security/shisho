//! This module defines `scan` subcommand.

use crate::cli::CommonOpts;
use structopt::StructOpt;

/// `Opts` defines possible options for the `scan` subcommand.
#[derive(StructOpt, Debug)]
pub struct Opts {
    target_path: String,
}

pub fn run(_common_opts: CommonOpts, _opts: Opts) -> i32 {
    todo!("not implemented yet")
}
