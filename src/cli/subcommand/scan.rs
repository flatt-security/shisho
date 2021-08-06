//! This module defines `scan` subcommand.

use std::path::PathBuf;

use crate::{
    cli::CommonOpts,
    language::{Go, Queryable, HCL},
    matcher::MatchedItem,
    ruleset::{self, QueryPattern, Rule},
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
    let queries = rule
        .patterns
        .iter()
        .map(|x| x.to_query::<T>())
        .collect::<Result<Vec<QueryPattern<T>>>>()?;

    // TODO: use all queries here
    assert_eq!(queries.len(), 1);
    let query = queries.get(0).unwrap();
    let mitems = match query {
        QueryPattern::Match(q) => {
            let session = tree.matches(q);
            session.collect()
        }
    };
    Ok(mitems)
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
            }
        };
    }

    Ok(())
}
