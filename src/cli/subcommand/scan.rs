//! This module defines `scan` subcommand.

use crate::{
    cli::CommonOpts,
    language::HCL,
    query::Query,
    tree::{QueryCursor, Tree},
};
use std::convert::TryFrom;
use structopt::StructOpt;

/// `Opts` defines possible options for the `scan` subcommand.
#[derive(StructOpt, Debug)]
pub struct Opts {
    target_path: String,
}

const CODE: &str = r#"resource "google_container_cluster" "test" {
    name     = "challenges"
    location = var.location
  
    initial_node_count    = 1
    enable_shielded_nodes = true
  
    addons_config {
      network_policy_config {
        disabled = false
      }
    }
  
    network_policy {
      enabled  = true
      provider = "CALICO"
    }
  
    cluster_autoscaling {
      enabled = true
      resource_limits {
        resource_type = "cpu"
        minimum       = 3
        maximum       = 32
      }
  
      resource_limits {
        resource_type = "memory"
        minimum       = 3
        maximum       = 32
      }
    }
  
    workload_identity_config {
      identity_namespace = "${var.project}.svc.id.goog"
    }
  
    network    = module.challenge_network.network_self_link
    subnetwork = element(module.challenge_network.subnets_self_links, 0)
  
    node_config {
      image_type      = "cos_containerd"
      service_account = var.node_service_account.email
      oauth_scopes    = ["cloud-platform"]
      machine_type    = var.machine_type
    }
  
    # Disable Basic Authentication to k8s API by setting empty value to `username`.
    master_auth {
      username = "a"
      password = ""
    }
  }
"#;

const QUERY: &str = r#"resource "google_container_cluster" "test" {
  ...
  master_auth {
    ...
    username = $X
    ...
  }
  ...
}
"#;

pub fn run(_common_opts: CommonOpts, _opts: Opts) -> i32 {
    // handle code
    let raw_code = CODE;
    let tree: Tree<HCL> = Tree::try_from(raw_code).expect("failed to load code");

    // handle query
    let raw_query = QUERY;
    let query = Query::try_from(raw_query).expect("failed to load query.");

    let cursor = &mut QueryCursor::new();
    for matched in tree.matches(&query, cursor) {
        // matched.
        println!("matched: {}", matched.pattern_index);

        for capture in matched.captures {
            println!("\t- {}: {:?}", capture.index, capture.node);
        }
    }

    // todo!("not implemented yet");
    return 0;
}
