//! This module defines options of `shisho` command.

use super::subcommand::*;
use crate::cli::reporter::ReporterType;
use clap_verbosity_flag::Verbosity;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(subcommand)]
    pub sub_command: SubCommand,
}

#[derive(StructOpt, Debug)]
pub struct CommonOpts {
    #[structopt(flatten)]
    pub verbose: Verbosity,
}

#[derive(StructOpt, Debug)]
pub struct ReportOpts {
    #[structopt(long, default_value = "console", possible_values(&ReporterType::variants()))]
    pub format: ReporterType,
}

#[derive(StructOpt, Debug)]
pub enum SubCommand {
    Completion(completion::CompletionOpts),
    Find(find::FindOpts),
    Check(check::CheckOpts),
}
