#[cfg(test)]
mod tests {
    use crate::cli::opts;
    use crate::cli::subcommand;
    use crate::reporter::ReporterType;
    use clap_verbosity_flag::Verbosity;
    use std::path::PathBuf;

    macro_rules! ruleset_test {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                for (lvalue, rvalue, mnum) in $value {
                    let mut ruleset = PathBuf::from(file!());
                    ruleset.pop();
                    ruleset.push("ruleset");
                    ruleset.push(stringify!($name));
                    ruleset.push(lvalue);

                    let mut target = PathBuf::from(file!());
                    target.pop();
                    target.push("ruleset");
                    target.push(stringify!($name));
                    target.push(rvalue);

                    let r = subcommand::check::run( subcommand::check::CheckOpts{
                        common: opts::CommonOpts { verbose: Verbosity::new(0, 0, 0) },
                        report: opts::ReportOpts { format: ReporterType::Console, },
                        ruleset_path: ruleset,
                        target_path: Some(target),
                    });
                    assert_eq!(r, mnum);
                }
            }
        )*
        }
    }

    ruleset_test! {
        unencrypted_ebs: [("ruleset.yaml", "match.tf", 1), ("ruleset.yaml", "unmatch.tf", 0)],
    }
}
