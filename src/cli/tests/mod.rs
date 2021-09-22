#[cfg(test)]
mod tests {
    use crate::cli::opts;
    use crate::cli::reporter::ReporterType;
    use crate::cli::subcommand;
    use anyhow::Result;
    use clap_verbosity_flag::Verbosity;
    use std::path::PathBuf;

    macro_rules! ruleset_test {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                for (lvalue, rvalue, mitem_num, encoding) in $value {
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

                    let mitem_num: Result<usize> = mitem_num;
                    let r = subcommand::check::handle_opts(subcommand::check::CheckOpts{
                        common: opts::CommonOpts { verbose: Verbosity::new(0, 0, 0) },
                        report: opts::ReportOpts { format: ReporterType::Console, },
                        ruleset_path: ruleset,
                        encoding: encoding,
                        target_path: Some(target),
                    });
                    match (r, mitem_num) {
                        (Ok(x), Ok(y)) if x == y => (),
                        (Err(x), Err(y)) if x.to_string() == y.to_string() => (),
                        (x, y) => panic!("[{}] {} + {}: expected {:?}, got {:?}", stringify!($name), lvalue, rvalue, y, x),
                    }
                }
            }
        )*
        }
    }

    ruleset_test! {
        unencrypted_ebs: [("ruleset.yaml", "match.tf", Ok(2), None), ("ruleset.yaml", "unmatch.tf", Ok(0), None)],
        uncontrolled_ebs_encryption_key: [("ruleset.yaml", "match.tf", Ok(2), None), ("ruleset.yaml", "unmatch.tf", Ok(0), None)],
        encoding: [
            ("ruleset.yaml", "shift_jis.go", Result::Ok(3), Some(encoding_rs::SHIFT_JIS)),
            ("ruleset.yaml", "utf_16le.go", Result::Ok(3), Some(encoding_rs::UTF_16LE)),
        ],
    }
}
