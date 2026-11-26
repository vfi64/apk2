# apk2 acceptance examples

This repository contains eleven executable contract examples for the planned
Rust implementation. Python's standard library runs the examples against local
disposable Git repositories. Git and Python 3 are the only prerequisites.

```sh
python3 -m unittest discover -s tests -v
```

The examples establish the following acceptance targets.

| Contract | Successful control | Counterexample |
| --- | --- | --- |
| Exact candidate | Selected source and its dependency pass in a clean tree | Working tree passes but candidate missing the dependency fails |
| Foreign staging | Selected commit contains its tested tree and foreign staging survives | Foreign file must retain its baseline contents in the commit |
| Owner decision | A work-branch note matches an independently supplied approval receipt | Missing receipt or expanded scope cannot establish approval |
| Result handling | A complete successful check permits integration | Failed check, crashed checker or absent evidence prevents integration |

These examples verify fixture behavior and Git mechanics. They do not certify an
apk2 executable. Once an executable exists, its normal entry points must satisfy
the same observations, including tests that fail against a deliberately defective
implementation. The two small reference predicates make the expected decision
rules executable without defining a public CLI or a persistent data schema.

The owner-decision example proposes a clarification to specification version 3.
A decision explicitly granted during an open task may be carried on its work
branch. Its authority comes from an independently verifiable owner grant. The
main branch records integrated project decisions. A self-authored note supplies
context but cannot authenticate its own approval. The host's authentication and
scope binding still need a real integration test.

The result example assumes a checker that documents exit 1 as a completed failed
check and other nonzero codes as execution errors. Future adapters must honor
their own documented checker contracts. Missing mandatory evidence always stops
integration.

No remote repository or credentials are needed for these tests. GitHub authority,
push recovery, concurrency, malicious project code and distribution installation
need separate acceptance coverage when those capabilities are introduced.

This package follows the owner-approved four-contract slice from 26 November
2026. It supplies requirements for the next prototype. Product benefit,
time/token savings and an independently assessed chat switch remain unmeasured.

## Confirmed local-first scope

The owner confirmed in the Codex chat on 26 November 2026 that the first
prototype prepares and checks changes locally. The owner transfers proposals
to GitHub. An apk2-owned push, its operation journal and push recovery are
deferred until a concrete need is demonstrated. This decision limits apk2's
features; it does not isolate an agent running with the owner's credentials.

The owner reported a successful local run of all four examples. A subsequent
agent-run verification at bd5182a also passed all four tests on macOS.
Repository identity and foreign-worktree reference examples have now passed
locally. Linux execution and an independently assessed chat switch remain
unverified.

## Repository discovery examples and Linux check

Four additional examples cover two real local clones with differently spelled
GitHub remotes, a different repository, unsupported remote formats, an ordinary
subdirectory, an independently initialized nested repository, and an ignored
child inside a foreign working tree. All remote names are fixture data; the
tests perform no network requests.

The reference identity comparison supports HTTPS, scp-style, and
`ssh://git@github.com/OWNER/REPO.git` URLs. Omitting the SSH port or specifying
`:22` yields the same identity as the equivalent HTTPS and scp-style URLs.
Other explicit SSH ports, other hosts, and unsupported formats are rejected.
The future Rust resolver needs its own normal-entry-point tests for this scope.
Root discovery compares Git's result with an independently supplied expected
workspace root. A caller must not infer the expected root from the same unchecked
discovery result.

The foreign-child example first observes a successful empty Git file listing
despite a real file being present, then requires rejection of the foreign root.
The nested-repository example confirms the inner root is accepted for the inner
workspace and rejected when the outer workspace was expected. These are mutation
boundary examples. Diagnostic file-scan fallback remains a separate behavior.

The workflow in .github/workflows/contracts.yml runs the full test suite on
Ubuntu with Python 3.11. It grants read-only repository permission and disables
persisted checkout credentials. The owner transfers the local commits, then
checks the Acceptance contracts workflow on the updated PR. Workflow success
provides Linux evidence for these examples. Enforcement as a required check and
isolation from owner credentials require separate host configuration.

Local preparation used the existing unittest harness and adds no dependencies.
The independent chat-switch assessment remains open. The owner must keep its
answer key outside the agent's accessible context.
