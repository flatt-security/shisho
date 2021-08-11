//! This module defines `scan` subcommand.

use crate::{
    cli::CommonOpts,
    language::{Go, Queryable, HCL},
    matcher::MatchedItem,
    query::Query,
    ruleset::{self, Rule},
    tree::{PartialTree, Tree},
};
use anyhow::{anyhow, Result};
use std::convert::TryFrom;
use std::path::PathBuf;
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

fn find_with_rule<'tree, 'item, T: 'static>(
    tree: &'tree PartialTree<'tree, 'tree, T>,
    rule: &Rule,
) -> Result<Vec<MatchedItem<'item>>>
where
    T: Queryable,
    'tree: 'item,
{
    let query = Query::<T>::try_from(rule.pattern.as_str())?;
    let session = tree.matches(&query);
    Ok(session.collect())
}

fn show_items<'tree, 'item, T: 'static>(
    tree: &'tree Tree<'tree, T>,
    items: &Vec<MatchedItem<'item>>,
) where
    T: Queryable,
    'tree: 'item,
{
    for mitem in items {
        println!("- item: {:?}", mitem.top.utf8_text(tree.raw).unwrap());
        for (id, c) in &mitem.captures {
            println!("\t- {}: {}", id.0, c.node.utf8_text(tree.raw).unwrap());
        }
    }
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
                let tree = Tree::<HCL>::try_from(file).unwrap();
                let ptree = tree.to_partial();
                let items = find_with_rule::<HCL>(&ptree, &rule)?;
                show_items(&tree, &items);
                unimplemented!("should be implemented before the first release")
            }
            ruleset::Language::Go => {
                let tree = Tree::<Go>::try_from(file).unwrap();
                let ptree = tree.to_partial();
                let items = find_with_rule::<Go>(&ptree, &rule)?;
                show_items(&tree, &items);
                unimplemented!("should be implemented before the first release")
            }
        };
    }

    Ok(())
}
