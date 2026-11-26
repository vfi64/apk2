# Task: SSH URL repository identity

Date: 2026-11-26
Base: main at fd5d9da13ad4bbb273eba9475f70a2cf103a9e1c
Work branch: codex/ssh-repository-identity
Status: reproduction recorded; awaiting the owner's port decision.

## Scope and authorization

Source: the owner's request in the Codex chat on 26 November 2026.
Authorized now: a local work branch, tests, this task note and local commits.
Behavior changes require the owner's port decision first. The owner handles
transfer to GitHub and merge. No new dependencies are allowed.
This note records the request; it does not authenticate or extend approval.

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

## Open owner decision and stop boundary

How should explicit SSH ports such as :22 and :2222 be handled?
- Reject all explicit ports.
- Treat :22 as the portless identity and reject other ports.
- Allow other ports too; specify whether they are ignored or part of identity.

No port policy has been selected or encoded in tests. Implementation is pending.
Stop here after reporting the reproduction and asking the owner. A later decision
must be recorded from the actual owner response; this note grants no permission
to implement, push or merge.
