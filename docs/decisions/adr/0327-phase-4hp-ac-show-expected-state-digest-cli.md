# ADR-0327: Phase 4HP Acceptance Criterion Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs ac show` renders an Acceptance Criterion semantic snapshot at either a
branch head or an explicit commit, including its state digest. Verification
tooling needs a direct assertion that a criterion snapshot is the expected
version before binding requirements or evidence.

## Decision

`workvcs ac show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Acceptance Criterion snapshot through the Engine facade, parses the expected
digest through the core `Digest` parser, and returns `matches_expected=true`
only when the Acceptance Criterion state digest matches. A mismatch returns
`DigestInvalid`.

## Consequences

Scripts can fail fast when an Acceptance Criterion id resolves to an unexpected
statement, classification, or requirement set. The command does not change
Acceptance Criterion creation, revision, list filtering, replay, or state
digest semantics.
