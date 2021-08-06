//! This module defines `scan` subcommand.

use std::path::PathBuf;

use crate::{
    cli::CommonOpts,
    language::{Go, Queryable, HCL},
    matcher::MatchedItem,
    query::Pattern,
    ruleset::{self, Rule},
    tree::{RawTree, Tree},
};
use anyhow::{anyhow, Result};
use structopt::StructOpt;

/// `Opts` defines possible options for the `scan` subcommand.
#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(parse(from_os_str))]
    target_path: PathBuf,

    #[structopt(parse(from_os_str))]
    ruleset_path: PathBuf,
}

pub fn run(common_opts: CommonOpts, opts: Opts) -> i32 {
    match intl(common_opts, opts) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn execute_rule<'tree, 'item, T: 'static>(
    tree: &'tree Tree<'tree, T>,
    rule: &Rule,
) -> Result<Vec<MatchedItem<'item>>>
where
    T: Queryable,
    'tree: 'item,
{
    let query = Pattern::<T>::new(&rule.pattern).to_query()?;
    let session = tree.matches(&query);
    Ok(session.collect())
}

// TODO (y0n3uchy): this is just a sample implementation! we need to improve this
fn intl(_common_opts: CommonOpts, opts: Opts) -> Result<()> {
    let ruleset = ruleset::from_reader(&opts.ruleset_path).map_err(|e| {
        anyhow!(
            "failed to load ruleset file {}: {}",
            opts.ruleset_path.as_os_str().to_string_lossy(),
            e
        )
    })?;

    // TODO: commonize the implementation with appropriate generics wrapper
    let file = std::fs::read_to_string(&opts.target_path).unwrap();
    let file = file.as_str();

    for rule in ruleset.rules {
        match rule.language {
            ruleset::Language::HCL => {
                let tree = RawTree::<HCL>::new(file).into_tree().unwrap();
                let items = execute_rule::<HCL>(&tree, &rule)?;
                for mitem in items {
                    println!("- item:");
                    for (id, c) in mitem.captures {
                        println!("\t- {}: {}", id.0, c.node.utf8_text(tree.raw).unwrap());
                    }
                }
                unimplemented!("should be implemented before the first release")
            }
            ruleset::Language::Go => {
                let tree = RawTree::<Go>::new(file).into_tree().unwrap();
                let items = execute_rule::<Go>(&tree, &rule)?;
                for mitem in items {
                    println!("- item:");
                    for (id, c) in mitem.captures {
                        println!("\t- {}: {}", id.0, c.node.utf8_text(tree.raw).unwrap());
                    }
                }
                unimplemented!("should be implemented before the first release")
            }
        };
    }

    Ok(())
}
