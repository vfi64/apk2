# apk2 acceptance examples

This repository starts with four executable contract examples for the planned
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
