# Task: SSH URL repository identity

Date: 2026-11-26
Base: main at fd5d9da13ad4bbb273eba9475f70a2cf103a9e1c
Work branch: codex/ssh-repository-identity
Status: owner decision recorded; subsequent implementation authorized; stopped
before implementation for chat switch.

## Scope and authorization

Source: the owner's original request and explicit option-2 follow-up decision
in the Codex chat on 26 November 2026.
The owner authorizes subsequent implementation within the decision below.
Local changes, tests and commits remain allowed. For this turn, only this note
may be changed and committed; then stop for chat switch before implementation.
The owner still handles transfer to GitHub and merge. No new dependencies.
This note records the owner's messages; it does not authenticate or extend approval.

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

## Recorded owner decision and chat-switch boundary

The owner chose option 2 and explicitly approved subsequent implementation:

- Portless SSH URLs and SSH URLs with explicit :22 yield the same repository
  identity, e.g. ssh://git@github.com/vfi64/apk2.git and
  ssh://git@github.com:22/vfi64/apk2.git both map to github.com/vfi64/apk2.
- All other explicit ports, including :2222, are rejected.
- Existing HTTPS/scp cases must keep passing. Equivalent spellings share identity;
  different repositories remain distinguishable. Existing tests are retained.

Reproduction commit: 693c997a726133295db013d8435482c96406ec90.
The reference function and tests have not been changed since that reproduction.
The results above are from that run; tests are not rerun for this note-only update.
Implementation and port-specific tests remain pending for the successor chat.
Next: add tests for the approved port policy, implement the bounded extension,
and run the existing unittest suite. Stop this turn after committing only this note.
