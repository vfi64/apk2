# Task: SSH URL repository identity

Date: 2026-11-26
Base: main at fd5d9da13ad4bbb273eba9475f70a2cf103a9e1c
Work branch: codex/ssh-repository-identity
Status: reference examples implemented and verified locally. GitHub transfer
and merge remain with the owner.

## Scope and authorization

After the chat switch, the owner confirmed that the chat-switch check passed and
authorized the two port tests, the bounded reference implementation, the README
and task note updates, the full local suite, and a local commit. The owner handles
GitHub transfer and merge. The untracked `.DS_Store` is outside the task.
The change uses the existing Python standard library.

## Work completed

Read README.md, tests/test_repository_contracts.py (including github_identity,
its caller and tests), tests/test_acceptance_contracts.py and the CI workflow.
Added test_ssh_url_shares_identity_with_https_and_scp without changing existing
tests or reference functions. It requires the portless SSH URL to share the
HTTPS/scp identity, while different repository names and owners stay distinct.

Verification: `python3 -B -m unittest discover -s tests -v`.
Before the addition: 8 tests passed, exit 0.
After the addition: 9 tests ran, 8 passed and the new test errored, exit 1.
Expected error at the first SSH call: `ValueError: unsupported GitHub remote`.
The later assertions in the new test are not reached yet. No successful SSH
behavior or Linux execution is claimed.

## Recorded owner decision

The owner chose option 2 and explicitly approved subsequent implementation:

- Portless SSH URLs and SSH URLs with explicit :22 yield the same repository
  identity, e.g. ssh://git@github.com/vfi64/apk2.git and
  ssh://git@github.com:22/vfi64/apk2.git both map to github.com/vfi64/apk2.
- All other explicit ports, including :2222, are rejected.
- Existing HTTPS/scp cases must keep passing. Equivalent spellings share identity;
  different repositories remain distinguishable. Existing tests are retained.

Reproduction commit: 693c997a726133295db013d8435482c96406ec90.

## Implementation and verification

Added separate tests for identity equivalence with explicit port 22 and for
rejection of ports 21, 23, 443, and 2222. With the new tests and the original
reference function, 9 of 11 tests passed. The portless and explicit-port-22
tests raised the expected `ValueError`. The rejection test already passed.

Extended the existing matcher to recognize the SSH URL prefix with an optional
`:22` and use the same repository normalization as HTTPS and scp-style URLs.
`python3 -B -m unittest discover -s tests -v` then passed all 11 tests on macOS.
Linux CI and a future Rust resolver remain outside this verification.
