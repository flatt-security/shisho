//! This module defines options of `shisho` command.

use super::subcommand::*;
use clap_verbosity_flag::Verbosity;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(flatten)]
    pub common_opts: CommonOpts,

    #[structopt(subcommand)]
    pub sub_command: SubCommand,
}

#[derive(StructOpt, Debug)]
pub struct CommonOpts {
    #[structopt(flatten)]
    pub verbose: Verbosity,
}

#[derive(StructOpt, Debug)]
pub enum SubCommand {
    Completion(completion::Opts),
}
