use shisho::cli;
use structopt::StructOpt;

/// `main` is an entrypoint of shisho.
fn main() {
    let opts: cli::Opts = cli::Opts::from_args();

    let exit_code = match opts.sub_command {
        cli::SubCommand::Completion(opts) => cli::subcommand::completion::run(opts),
        cli::SubCommand::Check(opts) => cli::subcommand::check::run(opts),
        cli::SubCommand::Find(opts) => cli::subcommand::find::run(opts),
    };

    std::process::exit(exit_code)
}
