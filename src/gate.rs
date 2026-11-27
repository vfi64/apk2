use std::ffi::OsString;
use std::io;
use std::path::{Component, PathBuf};
use std::process::{Command, Stdio};

use crate::repository::{github_identity, is_object_id, Git};
use crate::scratch::Scratch;

pub struct Request {
    pub root: PathBuf,
    pub identity: String,
    pub base: String,
    pub paths: Vec<PathBuf>,
    pub checker: PathBuf,
    pub checker_args: Vec<OsString>,
}

pub struct Workspace {
    root: PathBuf,
    pub base: String,
}

pub enum State {
    Pass,
    Rejected,
    Error,
    Blocked,
}

pub struct Report {
    pub state: State,
    pub base: Option<String>,
    pub tree: Option<String>,
}

pub struct Failure {
    pub state: State,
    pub message: String,
}

impl Failure {
    pub fn blocked(message: impl Into<String>) -> Self {
        Self {
            state: State::Blocked,
            message: message.into(),
        }
    }
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self {
            state: State::Error,
            message: error.to_string(),
        }
    }
}

pub fn run(request: Request) -> Result<Report, Failure> {
    check(&resolve(&request)?, &request)
}

impl Workspace {
    pub fn git(&self) -> Git<'_> {
        Git::new(&self.root)
    }
}

pub fn resolve(request: &Request) -> Result<Workspace, Failure> {
    let root = request
        .root
        .canonicalize()
        .map_err(|error| Failure::blocked(format!("invalid expected workspace root: {error}")))?;
    let working_directory = std::env::current_dir()?;
    let discovered = Git::new(&working_directory).root().map_err(|error| {
        Failure::blocked(format!("cannot discover a Git working tree: {error}"))
    })?;
    if root != discovered {
        return Err(Failure::blocked(
            "discovered Git root differs from expected workspace",
        ));
    }

    let git = Git::new(&root);
    let origin = git
        .origin()
        .map_err(|error| Failure::blocked(format!("cannot resolve origin: {error}")))?;
    if github_identity(&origin).as_deref() != Some(request.identity.as_str()) {
        return Err(Failure::blocked(
            "unsupported origin or GitHub repository identity mismatch",
        ));
    }
    if !is_object_id(&request.base) {
        return Err(Failure::blocked("base must be a full Git commit object ID"));
    }
    let base = git
        .commit(&request.base)
        .map_err(|error| Failure::blocked(format!("invalid base commit: {error}")))?;
    if !base.eq_ignore_ascii_case(&request.base) {
        return Err(Failure::blocked("base object must be a commit"));
    }
    if request.paths.iter().any(|path| {
        path.as_os_str().is_empty()
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
    }) {
        return Err(Failure::blocked(
            "selected paths must be relative to the workspace root",
        ));
    }

    Ok(Workspace { root, base })
}

pub fn check(workspace: &Workspace, request: &Request) -> Result<Report, Failure> {
    let git = workspace.git();
    let scratch = Scratch::new(&workspace.root)?;
    let index = scratch.path().join("index");
    let tree = git.candidate(&workspace.base, &request.paths, &index)?;
    let checkout = scratch.path().join("checkout");
    let environment = scratch.path().join("environment");
    std::fs::create_dir(&checkout)?;
    std::fs::create_dir(&environment)?;
    git.materialize(&tree, &checkout)?;

    let status = Command::new(&request.checker)
        .args(&request.checker_args)
        .current_dir(&checkout)
        .env_clear()
        .env(
            "PATH",
            std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()),
        )
        .env("HOME", &environment)
        .env("TMPDIR", &environment)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status();

    let state = match status {
        Ok(status) => match status.code() {
            Some(0) => State::Pass,
            Some(1) => State::Rejected,
            _ => {
                eprintln!("apk2: checker terminated with {status}");
                State::Error
            }
        },
        Err(error) => {
            eprintln!("apk2: could not start checker: {error}");
            State::Error
        }
    };
    Ok(Report {
        state,
        base: Some(workspace.base.clone()),
        tree: Some(tree),
    })
}
