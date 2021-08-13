use anyhow::Result;
use std::path::PathBuf;

use crate::ruleset::Language;

pub struct Target {
    pub path: PathBuf,
    pub body: String,
}

impl Target {
    pub fn new(path: PathBuf) -> Result<Self> {
        let body = std::fs::read_to_string(&path)?;
        Ok(Target { path, body })
    }

    pub fn language(&self) -> Option<Language> {
        if let Ok(p) = self.path.canonicalize() {
            let ext = p.extension()?;
            match ext.to_str() {
                Some("go") => Some(Language::Go),
                Some("tf") => Some(Language::HCL),
                _ => None,
            }
        } else {
            None
        }
    }
}
