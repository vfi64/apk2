"""Executable contract examples. These exercise Git, not an apk2 implementation."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class AcceptanceContracts(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="apk2-contract-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.env = dict(os.environ, GIT_CONFIG_NOSYSTEM="1",
                        GIT_CONFIG_GLOBAL=os.devnull)
        self.git("init", "-q")
        self.git("config", "user.name", "Contract fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.write("calculate.py", "answer = 42\n")
        self.write("check.py", "from calculate import answer\nassert answer == 42\n")
        self.write("unrelated.txt", "original\n")
        self.git("add", "--", ".")
        self.git("commit", "-qm", "Baseline")
        self.base = self.git("rev-parse", "HEAD").strip()

    def git(self, *args, env=None):
        return subprocess.check_output(
            ["git", "-c", "core.hooksPath=" + os.devnull, *args],
            cwd=self.repo, env=env or self.env, text=True,
        )

    def write(self, name, content):
        (self.repo / name).write_text(content, encoding="utf-8")

    def candidate(self, paths):
        index = self.root / "candidate-index"
        env = dict(self.env, GIT_INDEX_FILE=str(index))
        self.git("read-tree", self.base, env=env)
        self.git("add", "--", *paths, env=env)
        return self.git("write-tree", env=env).strip()

    def check_tree(self, tree, name):
        destination = self.root / name
        destination.mkdir()
        env = dict(self.env, GIT_INDEX_FILE=str(self.root / (name + "-index")))
        self.git("read-tree", tree, env=env)
        self.git("checkout-index", "-a", "--prefix=" + str(destination) + "/", env=env)
        return subprocess.run(
            [sys.executable, "-I", "-B", "-c",
             "import sys; sys.path.insert(0, '.'); exec(open('check.py').read())"],
            cwd=destination, capture_output=True,
        ).returncode

    def test_exact_candidate_excludes_unselected_dependency(self):
        self.write("calculate.py", "from constants import answer\n")
        self.write("constants.py", "answer = 42\n")
        working = subprocess.run(
            [sys.executable, "-B", "check.py"], cwd=self.repo, capture_output=True,
        )
        self.assertEqual(working.returncode, 0)
        incomplete = self.candidate(["calculate.py"])
        complete = self.candidate(["calculate.py", "constants.py"])
        self.assertNotEqual(self.check_tree(incomplete, "incomplete"), 0)
        self.assertEqual(self.check_tree(complete, "complete"), 0)
        self.assertNotEqual(incomplete, complete)

    def test_candidate_commit_preserves_foreign_staging(self):
        self.write("unrelated.txt", "foreign staged change\n")
        self.git("add", "--", "unrelated.txt")
        index_before = (self.repo / ".git/index").read_bytes()
        self.write("calculate.py", "answer = 6 * 7\n")
        tree = self.candidate(["calculate.py"])
        self.assertEqual(self.check_tree(tree, "checked"), 0)
        commit = self.git("commit-tree", tree, "-p", self.base, "-m", "Selected change").strip()
        self.git("update-ref", "HEAD", commit, self.base)
        self.assertEqual(self.git("rev-parse", "HEAD^{tree}").strip(), tree)
        self.assertEqual(self.git("show", "HEAD:unrelated.txt"), "original\n")
        self.assertEqual((self.repo / ".git/index").read_bytes(), index_before)
        self.git("reset", "-q", "HEAD", "--", "calculate.py")
        self.assertEqual(self.git("diff", "--cached", "--name-only").strip(), "unrelated.txt")
        self.assertEqual((self.repo / "unrelated.txt").read_text(), "foreign staged change\n")

    def test_owner_decision_can_be_carried_on_work_branch(self):
        # The trusted receipt is supplied by the evaluator, outside agent data.
        trusted = {"decision": "reject unreadable files", "scope": "read-error fix"}
        self.git("branch", "baseline", self.base)
        self.write("task.json", json.dumps(trusted))
        self.git("add", "--", "task.json")
        self.git("commit", "-qm", "Record externally approved decision")
        observed = json.loads(self.git("show", "HEAD:task.json"))
        self.assertEqual(observed, trusted)
        self.assertNotIn("task.json", self.git("ls-tree", "--name-only", "baseline"))
        proposed = dict(observed, scope="all persistence changes")
        self.assertNotEqual(proposed, trusted)
        # A branch note alone cannot establish who authorized its contents.
        self.assertFalse(approval_matches(observed, None))
        self.assertTrue(approval_matches(observed, trusted))
        self.assertFalse(approval_matches(proposed, trusted))

    def test_only_complete_successful_evidence_allows_integration(self):
        for outcome, evidence, expected in [
            (0, True, "PASS"), (1, True, "REJECTED"),
            (2, True, "ERROR"), (-9, True, "ERROR"),
            (0, False, "ERROR"), (None, False, "ERROR"),
        ]:
            with self.subTest(outcome=outcome, evidence=evidence):
                state = classify_reference_result(outcome, evidence)
                self.assertEqual(state, expected)
                self.assertEqual(state == "PASS", outcome == 0 and evidence)


def approval_matches(note, trusted_receipt):
    """Example comparison only; receipt authentication belongs to the host."""
    return trusted_receipt is not None and note == trusted_receipt


def classify_reference_result(exit_code, evidence_present):
    """Example checker contract: exit 1 is a completed failed check."""
    if not evidence_present:
        return "ERROR"
    return {0: "PASS", 1: "REJECTED"}.get(exit_code, "ERROR")


if __name__ == "__main__":
    unittest.main()
