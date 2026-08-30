# ADR-0383: Phase 4JT Doctor Count Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs doctor` opens a Store, reports manifest metadata, runs the Engine-owned
integrity check, and can now fail closed with `--require-valid`. Phase 4JS made
the narrower `workvcs store integrity` command script-verifiable by adding
expected count flags. The combined doctor entrypoint needs the same count gates
so a single command can validate both manifest visibility and integrity coverage.

## Decision

The CLI adds optional count expectations to `workvcs doctor`:

- `--expected-checked-branches COUNT`
- `--expected-checked-commits COUNT`
- `--expected-checked-changesets COUNT`
- `--expected-checked-change-operations COUNT`
- `--expected-checked-changeset-causal-anchors COUNT`
- `--expected-checked-events COUNT`
- `--expected-checked-checkpoints COUNT`
- `--expected-invalid-checkpoints COUNT`

The command still opens the Store through `Engine`, reads `store_info`, and runs
`validate_integrity`. Passing checks append the corresponding
`*_match_expected=true` marker. A mismatch returns `QueryInvalid`.

## Consequences

The standard doctor command can serve as a stronger one-step local acceptance
gate without direct SQL or external output parsing. This does not change Store
schema, integrity algorithms, checkpoint validation, replay, or bundle behavior.
