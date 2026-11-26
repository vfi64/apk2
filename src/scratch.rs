use std::fs::{self, DirBuilder};
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(workspace: &Path) -> io::Result<Self> {
        let parent = std::env::temp_dir().canonicalize()?;
        if parent.starts_with(workspace) {
            return Err(io::Error::other(
                "temporary directory is inside the workspace",
            ));
        }
        for attempt in 0..100 {
            let path = parent.join(format!("apk2-gate-{}-{attempt}", std::process::id()));
            match DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::other(
            "could not allocate a private temporary directory",
        ))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "apk2: could not clean temporary directory {}: {error}",
                self.0.display()
            );
        }
    }
}
