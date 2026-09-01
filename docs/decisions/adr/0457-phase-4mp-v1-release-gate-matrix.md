# ADR-0457: Phase 4MP V1 Release Gate Matrix

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger records current implementation, smoke, and dogfood
coverage. After ADR-0456 closed the JSON error output gap, the ledger still had
no single release-oriented view that separated proven V1 scope from remaining
release blockers.

Without a gate matrix, later slices could keep closing narrow gaps while the
release decision remained implicit. The useful next step is a current,
auditable decision aid that maps ledger rows into release gates and names the
evidence still required before a V1 release-maturity claim.

## Decision

Add `docs/provenance/v1-release-gate-matrix.md` as the current release-maturity
gate matrix for WorkVCS V1/V0.1 dogfood.

The matrix records:

- the current release decision;
- the current evidence basis;
- gate states of `Pass`, `Partial`, and `Blocked`;
- whether each gate blocks a V1 release claim; and
- the next evidence required to change blocking gates.

The current decision is:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```

Future slices may change individual gate states only with fresh evidence from
current project authority, implementation, smoke, dogfood, and validation
records. Any release-candidate or release operation still requires explicit
user authorization.

## Non-Goals

- No product, schema, CLI, Store, or runtime behavior change.
- No V1 release, release candidate, tag, push, or deployment.
- No V2 scope expansion.
- No replacement of product, architecture, schema, or accepted ADR authority.
- No claim that current V0.1 dogfood is complete.

## Evidence

- `docs/provenance/v1-release-gate-matrix.md` maps the current readiness ledger
  into release gates and records the current non-ready decision.
- `docs/provenance/v1-readiness-ledger.md` points release-maturity readers to
  the matrix and removes the obsolete open action to create a final gate
  matrix.
- `docs/README.md` links the matrix and this ADR from the documentation index.

## Consequences

WorkVCS now has an explicit release-maturity decision aid. The result is a
more precise blocker list, not a release claim. V0.1 dogfood remains the active
path until blocking gates receive fresh pass evidence and a separately
authorized release-candidate validation is run.
