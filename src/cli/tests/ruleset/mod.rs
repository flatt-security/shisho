#[macro_export]
macro_rules! ruleset_test {
    ($($name:ident: $value:expr,)*) => {
    $(
        #[test]
        fn $name() {
            use crate::cli::opts;
            use crate::cli::reporter::ReporterType;
            use crate::cli::subcommand;
            use anyhow::Result;
            use clap_verbosity_flag::Verbosity;
            use std::path::PathBuf;

            for (lvalue, rvalue, mitem_num, encoding) in $value {
                let mut ruleset = PathBuf::from(file!());
                ruleset.pop();
                ruleset.push(stringify!($name));
                ruleset.push(lvalue);

                let mut target = PathBuf::from(file!());
                target.pop();
                target.push(stringify!($name));
                target.push(rvalue);

                let mitem_num: Result<usize> = mitem_num;
                let r = subcommand::check::handle_opts(subcommand::check::CheckOpts{
                    common: opts::CommonOpts { verbose: Verbosity::new(0, 0, 0) },
                    report: opts::ReportOpts { format: ReporterType::Console, },
                    ruleset_path: ruleset,
                    encoding: encoding,
                    target_path: Some(target),
                    exit_zero: false,
                    exclude: vec![],
                });
                match (r, mitem_num) {
                    (Ok(x), Ok(y)) if x == y => (),
                    (Err(_), Err(_)) => (),
                    (x, y) => panic!("[{}] {} + {}: expected {:?}, got {:?}", stringify!($name), lvalue, rvalue, y, x),
                }
            }
        }
    )*
    }
}

#[cfg(test)]
mod generic;

#[cfg(test)]
mod hcl;
