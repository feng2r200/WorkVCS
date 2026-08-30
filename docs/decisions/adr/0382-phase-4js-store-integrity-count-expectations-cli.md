# ADR-0382: Phase 4JS Store Integrity Count Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs store integrity` runs the Engine-owned integrity report and renders the
number of checked Branches, Commits, ChangeSets, ChangeOperations, causal
anchors, Events, Checkpoints, and invalid Checkpoints. Scripts need to assert
those counts directly after local verification.

## Decision

The CLI adds optional count expectations to `workvcs store integrity`:

- `--expected-checked-branches COUNT`
- `--expected-checked-commits COUNT`
- `--expected-checked-changesets COUNT`
- `--expected-checked-change-operations COUNT`
- `--expected-checked-changeset-causal-anchors COUNT`
- `--expected-checked-events COUNT`
- `--expected-checked-checkpoints COUNT`
- `--expected-invalid-checkpoints COUNT`

The command still runs the existing Engine integrity check. Passing checks
append the corresponding `*_match_expected=true` marker. A mismatch returns
`QueryInvalid`.

## Consequences

Store health checks become directly script-verifiable without direct SQL or
ad-hoc output parsing. This does not change integrity algorithms, replay,
checkpoint validation, Store schema, or the separate `doctor` command.
