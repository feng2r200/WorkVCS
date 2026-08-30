# ADR-0246: Phase 4EM Verification Cache List CLI

Status: Accepted
Date: 2026-08-30

## Context

`verification_applicability_cache` is recorded by
`workvcs verification cache-record` and inspected one verification at a time by
`workvcs verification cache-show`. ADR-0197 kept cache listing deferred, which
left operators without a branch-level view of recorded verification
applicability cache rows.

The cache remains an observed applicability projection for already recorded
verification/resource state. Listing cache rows must not recompute
applicability, run resource adapters, change branch heads, or introduce new
schema.

## Decision

The Engine exposes `verification_applicability_caches`, returning cache rows for
a branch with optional filtering by verification entity id and applicability.
Each returned row is loaded through the existing
`verification_applicability_cache` lookup path so row validation stays shared
with `cache-show`.

The CLI adds:

- `workvcs verification cache-list <STORE> --branch <BRANCH_ID>`
- optional `--verification <ENTITY_ID>`
- optional `--applicability <applicable|stale|unknown>`

The output includes the branch id, row count, verification entity id, evaluated
commit id, applicability, reason code, evaluated timestamp, and resource stamp
count for each cache row.

## Consequences

Operators can audit branch-level verification applicability cache contents and
narrow the view by verification or applicability state.

The command is read-only. It does not replace `cache-record`, does not infer
missing cache rows, and does not change the effective acceptance criterion or
verification projections.
