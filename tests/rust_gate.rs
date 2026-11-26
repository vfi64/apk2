use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

struct Repo {
    root: PathBuf,
    base: String,
}

impl Repo {
    fn new() -> Self {
        let number = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("apk2-tests-{}-{number}", std::process::id()));
        fs::create_dir(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(
            &root,
            &[
                "remote",
                "add",
                "origin",
                "ssh://git@github.com:22/VFI64/Apk2.git",
            ],
        );
        fs::write(root.join("selected"), "baseline\n").unwrap();
        fs::write(root.join("unrelated"), "baseline\n").unwrap();
        fs::write(root.join(".gitattributes"), "selected text eol=crlf\n").unwrap();
        git(&root, &["add", "--", "."]);
        git(&root, &["commit", "-qm", "Fixture baseline"]);
        let base = git(&root, &["rev-parse", "HEAD"]);
        Self {
            root: root.canonicalize().unwrap(),
            base,
        }
    }

    fn gate(&self, cwd: &Path, identity: &str, paths: &[&str], checker: &str) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_apk2"));
        command.current_dir(cwd).args([
            "gate",
            "--root",
            self.root.to_str().unwrap(),
            "--identity",
            identity,
            "--base",
            &self.base,
        ]);
        for path in paths {
            command.args(["--path", path]);
        }
        command
            .args(["--", "/bin/sh", "-c", checker])
            .output()
            .unwrap()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=apk2 fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn outcome(output: Output, code: i32, state: &str) -> String {
    let report = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        output.status.code(),
        Some(code),
        "report: {report}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        report.contains(&format!("\"state\":\"{state}\"")),
        "{report}"
    );
    report
}

#[test]
fn checks_only_the_selected_tree_and_preserves_foreign_staging() {
    let repo = Repo::new();
    fs::write(repo.root.join("selected"), "changed\n").unwrap();
    fs::write(repo.root.join("dependency"), "required\n").unwrap();
    fs::write(repo.root.join("unrelated"), "foreign staged\n").unwrap();
    git(&repo.root, &["add", "--", "unrelated"]);
    let index = fs::read(repo.root.join(".git/index")).unwrap();
    let checker = "test -f dependency && grep -qx baseline unrelated && grep -qx changed selected && test \"$(wc -c < selected)\" -eq 8";
    assert!(Command::new("/bin/sh")
        .args(["-c", "test -f dependency"])
        .current_dir(&repo.root)
        .status()
        .unwrap()
        .success());

    let bad = outcome(
        repo.gate(&repo.root, "github.com/vfi64/apk2", &["selected"], checker),
        1,
        "REJECTED",
    );
    let good = outcome(
        repo.gate(
            &repo.root,
            "github.com/vfi64/apk2",
            &["selected", "dependency"],
            &format!("{checker} && printf check > generated"),
        ),
        0,
        "PASS",
    );
    let expected = git(&repo.root, &["rev-parse", "HEAD"]);
    assert!(good.contains(&expected));
    let tree = |report: &str| {
        report
            .split("\"tree\":\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_owned()
    };
    assert_ne!(tree(&bad), tree(&good));
    assert_eq!(
        git(
            &repo.root,
            &["show", &format!("{}:dependency", tree(&good))]
        ),
        "required"
    );
    assert_eq!(
        git(&repo.root, &["show", &format!("{}:unrelated", tree(&good))]),
        "baseline"
    );
    let staged = git(&repo.root, &["diff", "--cached", "--name-only"]);
    assert_eq!(staged, "unrelated");
    assert_eq!(fs::read(repo.root.join(".git/index")).unwrap(), index);
    assert!(!repo.root.join("generated").exists());
    assert_eq!(git(&repo.root, &["rev-parse", "HEAD"]), repo.base);
}

#[test]
fn accepts_equivalent_github_origin_spellings_from_a_subdirectory() {
    let repo = Repo::new();
    let child = repo.root.join("child");
    fs::create_dir(&child).unwrap();
    for url in [
        "https://github.com/VFI64/Apk2.git",
        "git@github.com:vfi64/apk2.git",
        "ssh://git@github.com/vfi64/apk2.git",
        "ssh://git@github.com:22/vfi64/apk2.git",
    ] {
        git(&repo.root, &["remote", "set-url", "origin", url]);
        outcome(
            repo.gate(&child, "github.com/vfi64/apk2", &["selected"], "exit 0"),
            0,
            "PASS",
        );
    }
}

#[test]
fn blocks_a_foreign_root() {
    let repo = Repo::new();
    let nested = repo.root.join("nested");
    fs::create_dir(&nested).unwrap();
    git(&nested, &["init", "-q"]);
    outcome(
        repo.gate(&nested, "github.com/vfi64/apk2", &["selected"], "exit 0"),
        3,
        "BLOCKED",
    );
}

#[test]
fn blocks_a_different_github_identity_and_nonstandard_ssh_ports() {
    let repo = Repo::new();
    outcome(
        repo.gate(&repo.root, "github.com/other/apk2", &["selected"], "exit 0"),
        3,
        "BLOCKED",
    );
    git(
        &repo.root,
        &[
            "remote",
            "set-url",
            "origin",
            "ssh://git@github.com:2222/vfi64/apk2.git",
        ],
    );
    outcome(
        repo.gate(&repo.root, "github.com/vfi64/apk2", &["selected"], "exit 0"),
        3,
        "BLOCKED",
    );
}

#[test]
fn distinguishes_checker_error_from_a_failed_check() {
    let repo = Repo::new();
    outcome(
        repo.gate(&repo.root, "github.com/vfi64/apk2", &["selected"], "exit 2"),
        2,
        "ERROR",
    );
}

#[test]
fn refuses_a_tree_with_a_symlink_before_running_the_checker() {
    let repo = Repo::new();
    std::os::unix::fs::symlink("selected", repo.root.join("link")).unwrap();
    outcome(
        repo.gate(&repo.root, "github.com/vfi64/apk2", &["link"], "exit 0"),
        2,
        "ERROR",
    );
}
