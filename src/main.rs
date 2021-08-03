use shisho::cli;
use structopt::StructOpt;

/// `main` is an entrypoint of shisho.
fn main() {
    let opts: cli::Opts = cli::Opts::from_args();

    let exit_code = match opts.sub_command {
        cli::SubCommand::Completion(sub_opts) => {
            cli::subcommand::completion::run(opts.common_opts, sub_opts)
        }
        cli::SubCommand::Scan(sub_opts) => cli::subcommand::scan::run(opts.common_opts, sub_opts),
    };

    std::process::exit(exit_code)
}
