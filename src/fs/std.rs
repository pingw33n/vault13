use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader, Result};

use super::{Metadata, Provider};

pub fn new_provider<P: AsRef<Path>>(path: P) -> Result<Box<dyn Provider>> {
    Ok(Box::new(StdFileSystem::new(path)))
}

struct StdFileSystem {
    root: PathBuf,
}

impl StdFileSystem {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        StdFileSystem { root: root.as_ref().to_path_buf() }
    }

    fn to_fs_path(&self, path: &str) -> PathBuf {
        let mut r = PathBuf::new();
        r.push(&self.root);
        for s in path.split(['/', '\\']) {
            r.push(s);
        }
        r
    }
}

impl Provider for StdFileSystem {
    fn reader(&self, path: &str) -> Result<Box<dyn BufRead + Send>> {
        Ok(Box::new(BufReader::new(File::open(self.to_fs_path(path))?)))
    }

    fn metadata(&self, path: &str) -> Result<Metadata> {
        let len = self.to_fs_path(path).metadata()?.len();
        Ok(Metadata { len })
    }
}
