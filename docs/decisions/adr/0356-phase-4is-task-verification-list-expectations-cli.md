# ADR-0356: Phase 4IS Task Verification List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

Task execution and verification workflows rely on several read-only list
surfaces: `task list`, `ac list`, `vr list`, and `verification list`. Each
already supports scoped filters and limits, but automation needs direct count
assertions on the rendered result sets.

## Decision

The CLI adds the following optional count expectations:

- `workvcs task list --expected-tasks COUNT`
- `workvcs ac list --expected-criteria COUNT`
- `workvcs vr list --expected-requirements COUNT`
- `workvcs verification list --expected-verifications COUNT`

Each command applies existing filters, renders the same list output, and
appends the corresponding `*_match_expected=true` marker when the rendered
count equals the supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Task and verification automation can fail fast when scoped list results differ
from the expected shape. These commands do not change Task transitions,
Acceptance Criterion semantics, Verification Requirement semantics,
Verification recording, effective projection, or list filtering behavior.
