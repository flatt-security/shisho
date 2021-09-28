use anyhow::Result;
use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;
use std::{env, io::Read, path::PathBuf};
use walkdir::WalkDir;

use crate::core::ruleset::Language;

#[derive(Debug)]
pub struct Target {
    pub path: Option<PathBuf>,
    pub body: String,
}

impl Target {
    pub fn from(path: Option<PathBuf>, encoding: Option<&'static Encoding>) -> Result<Self> {
        if let Some(path) = path {
            let body_bytes = std::fs::read(&path)?;
            let mut decoder = DecodeReaderBytesBuilder::new()
                .encoding(encoding)
                .build(&body_bytes[..]);

            let mut body_string = String::new();
            decoder.read_to_string(&mut body_string)?;

            Ok(Target {
                path: Some(path),
                body: body_string,
            })
        } else {
            let mut decoder = DecodeReaderBytesBuilder::new()
                .encoding(encoding)
                .build(std::io::stdin());

            let mut body_string = String::new();
            decoder.read_to_string(&mut body_string)?;

            Ok(Target {
                path,
                body: body_string,
            })
        }
    }
    pub fn canonicalized_path(&self) -> String {
        if let Some(ref p) = self.path {
            let p = p.canonicalize().unwrap();
            p.to_string_lossy().to_string()
        } else {
            "/dev/stdin".to_string()
        }
    }

    pub fn relative_path(&self) -> String {
        if let Some(ref p) = self.path {
            let p = p.canonicalize().unwrap();
            let p = p.strip_prefix(env::current_dir().unwrap()).unwrap();
            p.to_string_lossy().to_string()
        } else {
            "/dev/stdin".to_string()
        }
    }

    pub fn iter_from(
        p: PathBuf,
        encoding: Option<&'static Encoding>,
    ) -> impl Iterator<Item = Self> {
        WalkDir::new(p)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| p.is_file())
            .map(move |p| Target::from(Some(p), encoding.clone()))
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