//! This module includes some sub-modules to provide command-line interface of shisho.

pub mod opts;
pub use self::opts::*;

pub mod logger;
pub mod subcommand;
