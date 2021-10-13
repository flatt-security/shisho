use anyhow::Result;
use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;
use itertools::Itertools;
use pathdiff::diff_paths;
use std::{
    env,
    io::Read,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use crate::core::ruleset::Language;

#[derive(Debug)]
pub struct Target {
    pub path: Option<PathBuf>,
    pub body: String,
}

#[derive(Debug)]
pub struct TargetLoader {
    exclude_path_pattern: Vec<glob::Pattern>,
    encoding: Option<&'static Encoding>,
}

impl TargetLoader {
    pub fn new(
        exclude_path_pattern: Vec<String>,
        encoding: Option<&'static Encoding>,
    ) -> Result<Self> {
        let exclude_path_pattern = exclude_path_pattern
            .into_iter()
            .map(|p| {
                let mut ps = vec![glob::Pattern::new(&p)
                    .map_err(|e| anyhow::anyhow!("failed to load exclude pattern: {}", e))];

                // TODO (y0n3uchy): fix this dirty hack to exclude `./bar/piyo.go` with a pattern `bar` (i.e. without ./)
                if !p.starts_with("./") && !p.starts_with('/') {
                    ps.push(
                        glob::Pattern::new(&format!("./{}", p))
                            .map_err(|e| anyhow::anyhow!("failed to load exclude pattern: {}", e)),
                    );

                    ps.push(
                        glob::Pattern::new(&format!(
                            "./{}{}",
                            p,
                            if p.ends_with('/') { "**" } else { "/**" }
                        ))
                        .map_err(|e| anyhow::anyhow!("failed to load exclude pattern: {}", e)),
                    );
                };

                // TODO (y0n3uchy): fix this dirty hack to exclude `bar/piyo.go` with a pattern `bar`
                if !p.ends_with('*') {
                    ps.push(
                        glob::Pattern::new(&format!(
                            "{}{}",
                            p,
                            if p.ends_with('/') { "**" } else { "/**" }
                        ))
                        .map_err(|e| anyhow::anyhow!("failed to load exclude pattern: {}", e)),
                    );
                };
                ps
            })
            .flatten()
            .collect::<Result<Vec<glob::Pattern>>>()?;

        Ok(TargetLoader {
            exclude_path_pattern,
            encoding,
        })
    }

    pub fn from(&self, p: PathBuf) -> Result<Vec<Target>> {
        if p.is_dir() {
            Ok(self.from_dir(p))
        } else if self.should_load(&p) {
            Ok(vec![self.from_file(p)?])
        } else {
            Ok(vec![])
        }
    }

    fn from_dir(&self, p: PathBuf) -> Vec<Target> {
        WalkDir::new(p)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| p.is_file() && self.should_load(p))
            .map(move |p| self.from_file(p))
            .filter_map(|e| e.ok())
            .collect()
    }

    fn from_file(&self, p: PathBuf) -> Result<Target> {
        let body_bytes = std::fs::read(&p)?;
        let mut decoder = DecodeReaderBytesBuilder::new()
            .encoding(self.encoding)
            .build(&body_bytes[..]);

        let mut body_string = String::new();
        decoder.read_to_string(&mut body_string)?;

        Ok(Target {
            path: Some(p),
            body: body_string,
        })
    }

    pub fn from_reader<R: std::io::Read>(&self, r: R) -> Result<Target> {
        let mut decoder = DecodeReaderBytesBuilder::new()
            .encoding(self.encoding)
            .build(r);

        let mut body_string = String::new();
        decoder.read_to_string(&mut body_string)?;

        Ok(Target {
            path: None,
            body: body_string,
        })
    }

    pub(crate) fn should_load(&self, p: &Path) -> bool {
        self.exclude_path_pattern
            .iter()
            .all(|gpattern| !gpattern.matches(p.as_os_str().to_str().unwrap()))
    }
}

impl Target {
    pub fn canonicalized_path(&self) -> String {
        if let Some(ref p) = self.path {
            let p = p.canonicalize().unwrap();
            p.to_string_lossy().to_string()
        } else {
            "/dev/stdin".to_string()
        }
    }

    pub fn relative_path(&self) -> String {
        self.relative_path_from(&env::current_dir().unwrap())
    }

    fn relative_path_from(&self, base: &Path) -> String {
        if let Some(ref p) = self.path {
            let p = p.canonicalize().unwrap();
            let p = diff_paths(p, base).unwrap();
            p.to_string_lossy().to_string()
        } else {
            "/dev/stdin".to_string()
        }
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
            Some("go") => return Some(Language::Go),
            Some("tf") => return Some(Language::HCL),
            _ => (),
        };

        if p.file_name()?
            .to_ascii_lowercase()
            .to_str()?
            .split('.')
            .contains(&"dockerfile")
        {
            return Some(Language::Dockerfile);
        }

        return None;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Target, TargetLoader};

    #[test]
    fn test_loader_exclusion() {
        let loader = TargetLoader::new(vec!["foo".into(), "bar/*".into()], None).unwrap();
        assert!(loader.should_load(&PathBuf::from("hoge.go")));
        assert!(loader.should_load(&PathBuf::from("piyo/hoge.go")));
        assert!(loader.should_load(&PathBuf::from("./hoge.go")));
        assert!(loader.should_load(&PathBuf::from("./piyo/hoge.go")));

        assert!(!loader.should_load(&PathBuf::from("foo/hoge.go")));
        assert!(!loader.should_load(&PathBuf::from("foo/bar/bar.go")));
        assert!(!loader.should_load(&PathBuf::from("./foo/hoge.go")));
        assert!(!loader.should_load(&PathBuf::from("./foo/bar/bar.go")));

        assert!(loader.should_load(&PathBuf::from("foobar/bar/bar.go")));
        assert!(loader.should_load(&PathBuf::from("./foobar/bar/bar.go")));

        assert!(!loader.should_load(&PathBuf::from("bar/aaa.go")));
        assert!(!loader.should_load(&PathBuf::from("./bar/aaa.go")));
    }

    #[test]
    fn test_relative_path() {
        {
            let t = Target {
                path: Some(PathBuf::from(format!("{}", file!()))),
                body: "".to_string(),
            };

            let p = t.relative_path_from(&PathBuf::from("/workdir/hoge"));
            assert_eq!(
                p,
                format!(
                    "../..{}",
                    PathBuf::from(file!())
                        .canonicalize()
                        .unwrap()
                        .to_str()
                        .unwrap()
                )
            );
        }
    }
}
