# ADR-0001: Verification Applicability and Resource Drift

- **Status:** Accepted
- **Accepted by:** explicit user confirmations for decisions 81 and 88–111
- **Confirmation turns:** `3e66d1a4-50be-4941-b300-df7443efcba2`,
  `67d5e0f4-7156-4a8c-ae5e-acb65b4566e3`,
  `071133db-f4a6-4107-b1c3-e11bd3ea9874`,
  `63f90d5b-99e4-4ba1-97a1-e07301098abc`, and
  `cb525af6-57cf-4bc0-9a28-40340885c81b`

**Subsequent resolution:**
[ADR-0006](0006-sqlite-physical-schema-v0.1.md) later closes the Physical DDL,
ID/digest, payload-storage, Adapter/scope-version, fingerprint, and
Applicability-stamp constraints that were deliberately Open when this ADR was
accepted. The Open list below is historical for ADR-0001.

## Context

The initial baseline made Verification part of Versioned Work State and
Evidence immutable provenance, but it did not decide whether re-verification
updated one logical Verification or created a new judgment. It also lacked a
deterministic model for deciding whether an earlier result still applies after
Resource or Work-State changes.

## Decision

1. Each Verification is an immutable judgment instance. Re-verification
   creates another Verification; elapsed time or a repeated method does not
   automatically supersede an earlier judgment.
2. A Verification's immutable `result` is distinct from its branch-sensitive,
   derived applicability. V1 applicability has `applicable`, `stale`, and
   `unknown` outcomes.
3. Applicability is computed from a structured Verification Basis containing
   Resource and/or Work-State inputs. Resource scope may be resource-wide,
   path-scoped, or an explicit artifact/input set. Work-State basis includes
   the WorkStateCommit at verification time plus explicit semantic
   dependencies.
4. Acceptance Criterion effective status is the deterministic projection
   `unverified`, `verified`, `failed`, `stale`, or `conflicted`. A mandatory
   criterion cannot be bypassed by an ordinary coordination `force`; any
   future waiver must be a separately defined semantic operation with explicit
   provenance.
5. An Acceptance Criterion may define zero or more required Verification
   Requirements. A Requirement has stable AC-local identity and describes what
   must be proven, not the command used. When Requirements exist, a
   Verification targets one Requirement; otherwise it may target the AC
   directly. A V1 judgment has one target and one result. Multiple judgments
   may reuse the same immutable Evidence.
6. WorkVCS Core depends on a Resource Adapter contract rather than Git-specific
   logic. A Resource has stable logical identity and a rebindable locator. An
   immutable ResourceObservation records an observed state. An Adapter provides
   deterministic scoped fingerprinting and difference reporting.
7. A Git-backed observation must represent the actual verified working state,
   including relevant uncommitted changes, rather than HEAD alone. An
   unavailable or incomparable Resource yields `unknown`. Rebinding a locator
   does not itself prove continuity.
8. Verification-time observations or fingerprints are persisted. Mechanical
   Resource drift and derived applicability do not create a WorkStateCommit.
   Resource and Work-State components combine conservatively: any `stale`
   component yields `stale`; otherwise any `unknown` yields `unknown`; only
   otherwise is the result `applicable`.

## Consequences

- Historical judgments remain true statements about what was observed, while
  current Task completion uses only the effective branch-sensitive projection.
- Resource adapters, not the core engine, own source-specific comparison rules.
- Evidence from one execution can support multiple single-target judgments
  without duplicating immutable artifacts.
- Replaying Work-State history does not require observing an external Resource.

## Deliberately not decided

- final CLI command names or protocol encoding;
- exact Resource path/glob normalization, symlink, case, rename, and deletion
  rules;
- which Resource adapter kinds ship first;
- persisted observation capture points other than Verification and explicit
  snapshots;
- physical tables, columns, indexes, ID format, digest algorithm, or payload
  encoding;
- the exact future semantic-waiver operation, if one is admitted.
