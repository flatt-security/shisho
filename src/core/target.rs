use anyhow::Result;
use std::{io::Read, path::PathBuf};
use walkdir::WalkDir;

use crate::core::ruleset::Language;

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

    pub fn iter_from(p: PathBuf) -> impl Iterator<Item = Self> {
        WalkDir::new(p)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| p.is_file())
            .map(|p| Target::from(Some(p)))
            .filter_map(|e| e.ok())
    }

    pub fn is_file(&self) -> bool {
        self.path.is_some()
    }

    pub fn language(&self) -> Option<Language> {
        let p = self.path.as_ref().and_then(|p| p.canonicalize().ok())?;
        let ext = if let Some(ext) = p.extension() {
            Some(ext)
        } else {
            p.file_name()
        }?;

        match ext.to_str() {
            Some("go") => Some(Language::Go),
            Some("tf") => Some(Language::HCL),
            Some("Dockerfile") => Some(Language::Dockerfile),
            _ => None,
        }
    }
}
