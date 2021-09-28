//! This module defines `completion` subcommand.

use std::io;
use structopt::clap::Shell;
use structopt::StructOpt;

/// Prints scripts for shell completion.
#[derive(StructOpt, Debug)]
pub enum CompletionOpts {
    /// For zsh
    Zsh,
    /// For bash
    Bash,
    /// For fish
    Fish,
}

pub fn run(opts: CompletionOpts) -> i32 {
    match opts {
        CompletionOpts::Bash => completion(Shell::Bash),
        CompletionOpts::Zsh => completion(Shell::Zsh),
        CompletionOpts::Fish => completion(Shell::Fish),
    };

    0
}

fn completion(s: Shell) {
    super::super::Opts::clap().gen_completions_to(env!("CARGO_PKG_NAME"), s, &mut io::stdout())
}
