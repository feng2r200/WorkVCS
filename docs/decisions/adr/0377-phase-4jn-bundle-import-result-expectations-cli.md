# ADR-0377: Phase 4JN Bundle Import Result Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle import-dir` records a bundle import attempt and reports the
preflight outcome and classification counts. Scripts need to assert that the
recorded attempt describes the expected import condition.

## Decision

The CLI adds optional expectations to `workvcs bundle import-dir`:

- `--expected-outcome OUTCOME`
- `--expected-payload-files COUNT`
- `--expected-payload-references COUNT`
- `--expected-exported-branch-heads COUNT`
- `--expected-branch-heads-already-present COUNT`
- `--expected-branch-heads-missing COUNT`
- `--expected-branch-heads-fast-forward COUNT`
- `--expected-branch-heads-diverged COUNT`

The command still records the import attempt through the existing Engine path.
Passing checks append `outcome_matches_expected=true` or the corresponding
`*_match_expected=true` count marker. A mismatch returns `QueryInvalid`.

## Consequences

Import attempt recording becomes directly script-verifiable without moving
Branch heads or applying bundle contents. Because `import-dir` records an
attempt before expectation checks complete, callers that need failure without a
record should run `bundle preflight-dir` first. This does not change preflight
classification, bundle application, import attempt persistence, or storage
schema.
