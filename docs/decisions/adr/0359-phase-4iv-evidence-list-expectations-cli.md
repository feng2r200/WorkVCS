# ADR-0359: Phase 4IV Evidence List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs evidence list` renders captured evidence records after kind,
source-session, content, media-type, and limit filters. Evidence-driven
automation needs a direct way to assert that the rendered result set has the
expected size.

## Decision

`workvcs evidence list` accepts optional `--expected-evidences COUNT`.

The command applies existing filters and limits, renders the same list output,
and appends `evidences_match_expected=true` when the rendered count equals the
supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Evidence validation scripts can fail fast when captured evidence results differ
from the expected shape. This command does not change Evidence creation,
content digest semantics, content filtering, session linkage, or list ordering.
