//! This module defines `completion` subcommand.

use std::io;
use structopt::clap::Shell;
use structopt::StructOpt;

use crate::cli::CommonOpts;

/// Prints scripts for shell completion.
#[derive(StructOpt, Debug)]
pub enum CompletionOpts {
    Zsh,
    Bash,
    Fish,
}

pub fn run(_common_opts: CommonOpts, opts: CompletionOpts) -> i32 {
    match opts {
        CompletionOpts::Bash => completion(Shell::Bash),
        CompletionOpts::Zsh => completion(Shell::Zsh),
        CompletionOpts::Fish => completion(Shell::Fish),
    };

    return 0;
}

fn completion(s: Shell) {
    super::super::Opts::clap().gen_completions_to(env!("CARGO_PKG_NAME"), s, &mut io::stdout())
}
