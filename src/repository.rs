use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub struct Git<'a> {
    directory: &'a Path,
}

impl<'a> Git<'a> {
    pub fn new(directory: &'a Path) -> Self {
        Self { directory }
    }

    fn command(&self) -> Command {
        let mut command = Command::new("git");
        command.current_dir(self.directory);
        for (key, _) in std::env::vars_os() {
            if key.as_bytes().starts_with(b"GIT_") {
                command.env_remove(key);
            }
        }
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .env("GIT_NO_LAZY_FETCH", "1");
        command
    }

    fn output(&self, command: &mut Command, action: &str) -> io::Result<Vec<u8>> {
        let output = command.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::other(format!("git {action}: {}", stderr.trim())));
        }
        Ok(output.stdout)
    }

    pub fn root(&self) -> io::Result<PathBuf> {
        let output = self.output(
            self.command().args(["rev-parse", "--show-toplevel"]),
            "rev-parse",
        )?;
        let path = output.strip_suffix(b"\n").unwrap_or(&output);
        Path::new(OsStr::from_bytes(path)).canonicalize()
    }

    pub fn origin(&self) -> io::Result<String> {
        let output = self.output(
            self.command().args(["remote", "get-url", "origin"]),
            "remote get-url",
        )?;
        let url = output.strip_suffix(b"\n").unwrap_or(&output);
        String::from_utf8(url.to_vec()).map_err(io::Error::other)
    }

    pub fn commit(&self, base: &str) -> io::Result<String> {
        let output = self.output(
            self.command()
                .args(["rev-parse", "--verify", &format!("{base}^{{commit}}")]),
            "rev-parse",
        )?;
        object_id(&output)
    }

    pub fn candidate(&self, base: &str, paths: &[PathBuf], index: &Path) -> io::Result<String> {
        let indexed = || {
            let mut git = self.command();
            git.env("GIT_INDEX_FILE", index);
            git
        };
        self.output(indexed().args(["read-tree", base]), "read-tree")?;
        self.output(
            indexed()
                .args(["--literal-pathspecs", "add", "--"])
                .args(paths),
            "add",
        )?;
        object_id(&self.output(indexed().arg("write-tree"), "write-tree")?)
    }

    pub fn materialize(&self, tree: &str, checkout: &Path) -> io::Result<()> {
        let listing = self.output(
            self.command().args(["ls-tree", "-rz", "--full-tree", tree]),
            "ls-tree",
        )?;
        // Read blobs directly so checkout filters cannot change the tested bytes.
        for entry in listing
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
        {
            let Some(tab) = entry.iter().position(|byte| *byte == b'\t') else {
                return Err(io::Error::other("invalid Git tree entry"));
            };
            let (metadata, path) = entry.split_at(tab);
            let path = Path::new(OsStr::from_bytes(&path[1..]));
            if path.as_os_str().is_empty() || path.components().any(|part| {
                !matches!(part, Component::Normal(name) if !name.as_bytes().eq_ignore_ascii_case(b".git"))
            }) {
                return Err(io::Error::other("unsafe path in Git tree"));
            }
            let mut fields = metadata.split(|byte| *byte == b' ');
            let permissions = match (fields.next(), fields.next()) {
                (Some(b"100644"), Some(b"blob")) => 0o644,
                (Some(b"100755"), Some(b"blob")) => 0o755,
                _ => return Err(io::Error::other(
                    "candidate contains an unsupported Git entry; only regular files are supported",
                )),
            };
            let blob = fields
                .next()
                .ok_or_else(|| io::Error::other("missing Git blob ID"))?;
            let blob = object_id(blob)?;
            let contents =
                self.output(self.command().args(["cat-file", "blob", &blob]), "cat-file")?;
            let destination = checkout.join(path);
            fs::create_dir_all(destination.parent().expect("path is below checkout"))?;
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(permissions)
                .open(destination)?
                .write_all(&contents)?;
        }
        Ok(())
    }
}

pub fn is_object_id(id: &str) -> bool {
    matches!(id.len(), 40 | 64) && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn object_id(output: &[u8]) -> io::Result<String> {
    let id = std::str::from_utf8(output.strip_suffix(b"\n").unwrap_or(output))
        .map_err(io::Error::other)?;
    if !is_object_id(id) {
        return Err(io::Error::other("invalid Git object ID"));
    }
    Ok(id.to_owned())
}

pub fn github_identity(url: &str) -> Option<String> {
    const PREFIXES: [&str; 4] = [
        "https://github.com/",
        "git@github.com:",
        "ssh://git@github.com/",
        "ssh://git@github.com:22/",
    ];
    let lower = url.to_ascii_lowercase();
    let prefix = PREFIXES.iter().find(|prefix| lower.starts_with(**prefix))?;
    let name = &url[prefix.len()..];
    let (owner, repo) = name.split_once('/')?;
    let valid = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
    };
    if !valid(owner) || !valid(repo) {
        return None;
    }
    let name = name.strip_suffix(".git").unwrap_or(name);
    Some(format!("github.com/{}", name.to_ascii_lowercase()))
}
