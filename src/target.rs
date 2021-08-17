use anyhow::Result;
use std::{io::Read, path::PathBuf};

use crate::ruleset::Language;

#[derive(Debug)]
pub struct Target {
    pub path: Option<PathBuf>,
    pub body: String,
}

impl Target {
    pub fn from(path: Option<PathBuf>) -> Result<Self> {
        if let Some(path) = path {
            let body = std::fs::read_to_string(&path)?;
            Ok(Target {
                path: Some(path),
                body,
            })
        } else {
            let mut body = String::new();
            std::io::stdin().read_to_string(&mut body)?;
            Ok(Target { path, body })
        }
    }

    pub fn is_file(&self) -> bool {
        self.path.is_some()
    }

    pub fn language(&self) -> Option<Language> {
        if let Some(p) = self.path.as_ref().and_then(|p| p.canonicalize().ok()) {
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
