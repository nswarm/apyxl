use std::env;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use chumsky::prelude::*;
use walkdir::WalkDir;

use crate::input;
use crate::input::{Chunk, Data, Input};

/// Input from one or more files in a file system.
#[derive(Default)]
pub struct Glob {
    file_set: input::FileSet,
}

impl Glob {
    pub fn new(glob: &str) -> Result<Self> {
        let (root, glob) = match split_glob(glob) {
            Some((prefix, glob)) => {
                if prefix.is_relative() {
                    (env::current_dir()?.join(prefix), glob)
                } else {
                    (prefix, glob)
                }
            }
            _ => (env::current_dir()?, glob.to_string()),
        };
        Self::new_with_root(root, &glob)
    }

    pub fn new_with_root<P: AsRef<Path>>(root_path: P, glob: &str) -> Result<Self> {
        let root_path = root_path.as_ref();
        let file_set = if !glob.is_empty() {
            input::FileSet::new(root_path, &walk_glob(root_path, glob)?)?
        } else if root_path.is_dir() {
            input::FileSet::new(root_path, &walk_glob(root_path, "**/*")?)?
        } else {
            input::FileSet::new(
                root_path.parent().unwrap(),
                &[root_path.file_name().unwrap()],
            )?
        };
        Ok(Self { file_set })
    }
}

impl Input for Glob {
    fn chunks(&self) -> Vec<(&Chunk, &Data)> {
        self.file_set.chunks()
    }
}

fn walk_glob(root: &Path, glob: &str) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let glob_path = root.join(glob);
    let glob = globset::Glob::new(
        glob_path
            .to_str()
            .ok_or_else(|| anyhow!("could not convert glob path '{:?}' to OS str", glob_path))?,
    )?
    .compile_matcher();
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }
        if !glob.is_match(entry.path()) {
            continue;
        }
        paths.push(entry.path().strip_prefix(root)?.to_path_buf());
    }
    Ok(paths)
}

/// Splits a glob into prefix path and glob.
/// e.g.
///     a/b/c/**/*.rs
/// would return
///     (PathBuf::from("a/b/c"), "**/*.rs")
fn split_glob(glob: &str) -> Option<(PathBuf, String)> {
    let prefix_parser = any::<&str, extra::Err<Cheap>>()
        .and_is(none_of("?*{}[]!"))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(PathBuf::from);
    let glob_parser = any().repeated().collect::<String>();
    let parser = prefix_parser.then(glob_parser).then_ignore(end());
    parser.parse(glob).into_output()
}

#[cfg(test)]
mod tests {
    mod new_with_root {
        use crate::input::Glob;
        use crate::Input;
        use anyhow::Result;
        use itertools::Itertools;
        use std::fs;
        use std::fs::File;
        use std::path::PathBuf;
        use tempfile::tempdir;

        #[test]
        fn glob() -> Result<()> {
            let root = tempdir()?;
            fs::create_dir_all(root.path().join("a/b"))?;
            fs::create_dir_all(root.path().join("a/c"))?;
            fs::create_dir_all(root.path().join("d/e"))?;
            let paths = [
                PathBuf::from("a/b/file0.rs"),
                PathBuf::from("a/b/file1.rs"),
                PathBuf::from("a/c/file2.rs"),
                PathBuf::from("d/e/file3.rs"),
            ];
            File::create(root.path().join(&paths[0]))?;
            File::create(root.path().join(&paths[1]))?;
            File::create(root.path().join(&paths[2]))?;
            File::create(root.path().join(&paths[3]))?;

            let glob = Glob::new_with_root(root.path().join("a"), "**/*.rs")?;
            assert_files(glob, vec!["b/file0.rs", "b/file1.rs", "c/file2.rs"]);
            Ok(())
        }

        #[test]
        fn directory() -> Result<()> {
            let root = tempdir()?;
            fs::create_dir_all(root.path().join("a/b"))?;
            let paths = [PathBuf::from("a/b/file0.rs"), PathBuf::from("a/b/file1.rs")];
            File::create(root.path().join(&paths[0]))?;
            File::create(root.path().join(&paths[1]))?;

            let glob = Glob::new_with_root(root.path().join("a/b"), "")?;
            assert_files(glob, vec!["file0.rs", "file1.rs"]);
            Ok(())
        }

        #[test]
        fn single_file() -> Result<()> {
            let root = tempdir()?;
            fs::create_dir_all(root.path().join("a/b"))?;
            let paths = [PathBuf::from("a/b/file0.rs")];
            File::create(root.path().join(&paths[0]))?;

            let glob = Glob::new_with_root(root.path().join("a/b/file0.rs"), "")?;
            assert_files(glob, vec!["file0.rs"]);
            Ok(())
        }

        fn assert_files(glob: Glob, mut expected: Vec<&str>) {
            let file_names = glob
                .chunks()
                .iter()
                .map(|(c, _)| {
                    c.relative_file_path
                        .as_ref()
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/")
                })
                .sorted()
                .collect_vec();
            expected.sort();
            assert_eq!(file_names, expected);
        }
    }

    mod split_glob {
        use std::path::PathBuf;

        use crate::input::glob::split_glob;

        #[test]
        fn path_and_glob() {
            assert_eq!(
                split_glob("a/b/c/**/*"),
                Some((PathBuf::from("a/b/c"), "**/*".to_string()))
            );
        }

        #[test]
        fn path_only() {
            assert_eq!(
                split_glob("a/b/c.rs"),
                Some((PathBuf::from("a/b/c.rs"), "".to_string()))
            );
        }

        #[test]
        fn glob_only() {
            assert_eq!(split_glob("**/asdf/*.rs"), None);
        }
    }
}
