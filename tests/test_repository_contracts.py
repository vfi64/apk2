"""Reference contracts for GitHub identity and Git working-tree discovery."""

import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest


class RepositoryContracts(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="apk2-repo-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.env = {key: value for key, value in os.environ.items()
                    if not key.startswith("GIT_")}
        self.env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
        self.parent = self.root / "parent"
        self.parent.mkdir()
        self.git(self.parent, "init", "-q")

    def git(self, path, *arguments):
        return subprocess.check_output(
            ["git", "-c", "core.hooksPath=" + os.devnull, *arguments],
            cwd=path, env=self.env, text=True, stderr=subprocess.PIPE,
        ).strip()

    def test_two_clones_share_identity_despite_url_case(self):
        identities = []
        for name, url in [
            ("first", "https://github.com/VFI64/Apk2.git"),
            ("second", "git@github.com:vfi64/apk2.git"),
        ]:
            clone = self.root / name
            self.git(self.root, "clone", "--no-hardlinks", str(self.parent), str(clone))
            self.git(clone, "remote", "set-url", "origin", url)
            identities.append(github_identity(self.git(clone, "remote", "get-url", "origin")))
        self.assertEqual(identities, ["github.com/vfi64/apk2"] * 2)
        self.assertNotEqual(identities[0], github_identity("https://github.com/vfi64/other.git"))
        for unsupported in ["https://elsewhere.example/vfi64/apk2.git", "../apk2"]:
            with self.subTest(url=unsupported), self.assertRaises(ValueError):
                github_identity(unsupported)

    def test_ssh_url_shares_identity_with_https_and_scp(self):
        identity = github_identity("ssh://git@github.com/VFI64/Apk2.git")
        self.assertEqual(identity, "github.com/vfi64/apk2")
        for equivalent in [
            "https://github.com/vfi64/apk2.git",
            "git@github.com:vfi64/apk2.git",
        ]:
            with self.subTest(url=equivalent):
                self.assertEqual(identity, github_identity(equivalent))
        for different in [
            "ssh://git@github.com/vfi64/other.git",
            "ssh://git@github.com/other-owner/apk2.git",
        ]:
            with self.subTest(url=different):
                self.assertNotEqual(identity, github_identity(different))

    def test_ssh_url_with_explicit_port_22_shares_identity(self):
        self.assertEqual(
            github_identity("ssh://git@github.com:22/vfi64/apk2.git"),
            github_identity("ssh://git@github.com/vfi64/apk2.git"),
        )

    def test_ssh_url_rejects_other_explicit_ports(self):
        for port in (21, 23, 443, 2222):
            with self.subTest(port=port), self.assertRaises(ValueError):
                github_identity(f"ssh://git@github.com:{port}/vfi64/apk2.git")

    def test_subdirectory_resolves_to_own_root(self):
        child = self.parent / "src"
        child.mkdir()
        self.assertEqual(expected_root(child, self.parent, self.env), self.parent)

    def test_nested_repository_resolves_to_inner_root(self):
        nested = self.parent / "nested"
        nested.mkdir()
        self.git(nested, "init", "-q")
        self.assertEqual(expected_root(nested, nested, self.env), nested)
        with self.assertRaises(ValueError):
            expected_root(nested, self.parent, self.env)

    def test_ignored_foreign_child_cannot_produce_false_success(self):
        (self.parent / ".gitignore").write_text("foreign/\n")
        foreign = self.parent / "foreign"
        foreign.mkdir()
        (foreign / "must-be-checked.txt").write_text("a real unchecked input\n")
        # Reproduce GF-105: successful Git invocation with an empty file list.
        self.assertEqual(self.git(foreign, "ls-files"), "")
        with self.assertRaises(ValueError):
            expected_root(foreign, foreign, self.env)
        with self.assertRaises(subprocess.CalledProcessError):
            expected_root(self.root, self.root, self.env)


def github_identity(url):
    """Normalize HTTPS, scp-style, and SSH URLs for GitHub remotes."""
    match = re.fullmatch(
        r"(?:https://github\.com/|git@github\.com:|ssh://git@github\.com(?::22)?/)"
        r"([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)",
        url, re.IGNORECASE,
    )
    if match is None:
        raise ValueError("unsupported GitHub remote")
    name = match.group(1)
    if name.endswith(".git"):
        name = name[:-4]
    return "github.com/" + name.lower()


def expected_root(path, expected, environment):
    """Bind Git discovery to an independently supplied workspace root."""
    observed = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], cwd=path,
        env=environment, text=True, stderr=subprocess.PIPE,
    ).strip()
    resolved = Path(observed).resolve()
    if resolved != Path(expected).resolve():
        raise ValueError("discovered root differs from expected workspace")
    return resolved


if __name__ == "__main__":
    unittest.main()
