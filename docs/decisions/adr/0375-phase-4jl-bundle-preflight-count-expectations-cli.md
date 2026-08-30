# ADR-0375: Phase 4JL Bundle Preflight Count Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle preflight-dir` reports payload directory counts and exported
Branch head classification counts before bundle import or apply. Scripts need
to assert those counts directly when checking whether a bundle is already
present, missing, fast-forwardable, or divergent.

## Decision

The CLI adds optional count expectations to `workvcs bundle preflight-dir`:

- `--expected-payload-files COUNT`
- `--expected-payload-references COUNT`
- `--expected-exported-branch-heads COUNT`
- `--expected-branch-heads-already-present COUNT`
- `--expected-branch-heads-missing COUNT`
- `--expected-branch-heads-fast-forward COUNT`
- `--expected-branch-heads-diverged COUNT`

The command still builds the preflight report through the existing Engine path.
When an expectation is supplied, the rendered count must match the expected
count. Passing checks append the corresponding `*_match_expected=true` marker.
A mismatch returns `QueryInvalid`.

## Consequences

Bundle import scripts can fail fast on unexpected preflight classification
without parsing policy outside WorkVCS. This does not change bundle validation,
manifest parsing, branch preflight classification, import attempt recording,
apply semantics, or storage schema.
