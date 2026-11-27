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
        git(&root, &["init", "-q", "-b", "source"]);
        git(&root, &["config", "user.name", "apk2 fixture"]);
        git(&root, &["config", "user.email", "fixture@example.invalid"]);
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

    fn propose(&self, name: &str, message: &str, checker: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_apk2"))
            .current_dir(&self.root)
            .args([
                "propose",
                "--root",
                self.root.to_str().unwrap(),
                "--identity",
                "github.com/vfi64/apk2",
                "--base",
                &self.base,
                "--source-ref",
                "refs/heads/source",
                "--proposal",
                name,
                "--message",
                message,
                "--path",
                "selected",
                "--",
                "/bin/sh",
                "-c",
                checker,
                "checker",
                self.root.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    }

    fn proposal_refs(&self) -> String {
        git(
            &self.root,
            &[
                "for-each-ref",
                "--format=%(refname)",
                "refs/apk2/proposals/",
            ],
        )
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

fn report_string(report: &str, field: &str) -> String {
    report
        .split(&format!("\"{field}\":\""))
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned()
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

#[test]
fn proposes_exact_checked_tree_and_preserves_source_checkout() {
    let repo = Repo::new();
    fs::write(repo.root.join("selected"), "checked candidate\n").unwrap();
    fs::write(repo.root.join("unrelated"), "foreign staged\n").unwrap();
    git(&repo.root, &["add", "--", "unrelated"]);
    let index = fs::read(repo.root.join(".git/index")).unwrap();
    let report = outcome(
        repo.propose(
            "trial",
            "Propose change",
            "grep -qx 'checked candidate' selected && grep -qx baseline unrelated",
        ),
        0,
        "PASS",
    );
    assert_eq!(report_string(&report, "outcome"), "DONE");
    let commit = report_string(&report, "commit");
    assert_eq!(
        git(&repo.root, &["rev-parse", "refs/apk2/proposals/trial"]),
        commit
    );
    assert_eq!(
        git(&repo.root, &["rev-parse", &format!("{commit}^{{tree}}")]),
        report_string(&report, "tree")
    );
    assert_eq!(
        git(&repo.root, &["show", "-s", "--format=%P", &commit]),
        repo.base
    );
    assert_eq!(
        git(&repo.root, &["show", &format!("{commit}:selected")]),
        "checked candidate"
    );
    assert_eq!(
        git(&repo.root, &["show", &format!("{commit}:unrelated")]),
        "baseline"
    );
    assert_eq!(git(&repo.root, &["rev-parse", "HEAD"]), repo.base);
    assert_eq!(fs::read(repo.root.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read_to_string(repo.root.join("selected")).unwrap(),
        "checked candidate\n"
    );
    assert_eq!(
        fs::read_to_string(repo.root.join("unrelated")).unwrap(),
        "foreign staged\n"
    );
}

#[test]
fn rechecks_identical_proposal_and_blocks_different_messages_or_trees() {
    let repo = Repo::new();
    fs::write(repo.root.join("selected"), "first\n").unwrap();
    let initial = outcome(
        repo.propose("trial", "Begin\n\nDetails", "exit 0"),
        0,
        "PASS",
    );
    let commit = report_string(&initial, "commit");
    outcome(
        repo.propose("trial", "Begin\n\nDetails", "exit 1"),
        1,
        "REJECTED",
    );
    let again = outcome(
        repo.propose("trial", "Begin\n\nDetails", "exit 0"),
        0,
        "PASS",
    );
    assert_eq!(report_string(&again, "outcome"), "NOOP");
    assert_eq!(report_string(&again, "commit"), commit);
    outcome(
        repo.propose("trial", "Different message", "exit 0"),
        3,
        "BLOCKED",
    );
    fs::write(repo.root.join("selected"), "second\n").unwrap();
    outcome(
        repo.propose("trial", "Begin\n\nDetails", "exit 0"),
        3,
        "BLOCKED",
    );
    assert_eq!(
        git(&repo.root, &["rev-parse", "refs/apk2/proposals/trial"]),
        commit
    );
}

#[test]
fn failed_or_crashed_checker_creates_no_proposal() {
    let repo = Repo::new();
    outcome(repo.propose("reject", "Change", "exit 1"), 1, "REJECTED");
    outcome(repo.propose("error", "Change", "exit 2"), 2, "ERROR");
    assert!(repo.proposal_refs().is_empty());
}

#[test]
fn refuses_to_publish_after_source_ref_moves_during_check() {
    let repo = Repo::new();
    let tree = git(&repo.root, &["rev-parse", "HEAD^{tree}"]);
    let advance = git(
        &repo.root,
        &[
            "commit-tree",
            &tree,
            "-p",
            &repo.base,
            "-m",
            "Concurrent advance",
        ],
    );
    let checker = format!(
        "git -C \"$1\" update-ref refs/heads/source {advance} {}",
        repo.base
    );
    outcome(
        repo.propose("stale", "Should not publish", &checker),
        3,
        "BLOCKED",
    );
    assert_eq!(
        git(&repo.root, &["rev-parse", "refs/heads/source"]),
        advance
    );
    assert!(repo.proposal_refs().is_empty());
}

#[test]
fn refuses_invalid_names_and_symbolic_proposal_refs() {
    let repo = Repo::new();
    outcome(repo.propose("../escaped", "Change", "exit 2"), 3, "BLOCKED");
    assert!(repo.proposal_refs().is_empty());
    git(
        &repo.root,
        &[
            "symbolic-ref",
            "refs/apk2/proposals/trial",
            "refs/heads/source",
        ],
    );
    outcome(repo.propose("trial", "Change", "exit 0"), 3, "BLOCKED");
    assert_eq!(
        git(&repo.root, &["symbolic-ref", "refs/apk2/proposals/trial"]),
        "refs/heads/source"
    );
    assert_eq!(git(&repo.root, &["rev-parse", "HEAD"]), repo.base);
}

#[test]
fn commits_the_checked_tree_even_if_file_changes_during_the_check() {
    let repo = Repo::new();
    fs::write(repo.root.join("selected"), "candidate\n").unwrap();
    let report = outcome(
        repo.propose(
            "trial",
            "Keep the checked snapshot",
            "grep -qx candidate selected && printf 'later edit\\n' > \"$1/selected\"",
        ),
        0,
        "PASS",
    );
    let commit = report_string(&report, "commit");
    assert_eq!(
        git(&repo.root, &["show", &format!("{commit}:selected")]),
        "candidate"
    );
    assert_eq!(
        fs::read_to_string(repo.root.join("selected")).unwrap(),
        "later edit\n"
    );
}

#[test]
fn similar_tag_names_do_not_shadow_proposal_refs() {
    let repo = Repo::new();
    git(
        &repo.root,
        &[
            "update-ref",
            "refs/tags/refs/apk2/proposals/trial",
            &repo.base,
        ],
    );
    outcome(repo.propose("trial", "Change", "exit 0"), 0, "PASS");
    assert_eq!(repo.proposal_refs(), "refs/apk2/proposals/trial");
}
