# ADR-0181: Phase 4BZ Claim Observability

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

The Runtime Coordination model already stores immutable Claim occurrences and
an active `claim_runtime` projection. Earlier slices exposed Claim creation,
release, claim-next, shared Claim creation, and shared next workflows. The CLI
still lacked a direct way to inspect one Claim or recover the active Claim ids
owned by a Session.

## Decision

1. Core adds `ClaimListOptions::for_session` and `ClaimListResult`.
2. `Engine::active_claims_for_session` returns the current active Claim
   snapshots for one existing Session.
3. `Engine::claim_snapshot` remains the authoritative read for one Claim
   occurrence, active or released.
4. CLI adds `claim show --claim` and `claim list --session`.
5. `claim list --session` reports only active ClaimRuntime rows for that
   Session. Released occurrences remain visible through `claim show`.

## Non-Goals

- This slice does not add a global Claim search or filtering DSL.
- This slice does not implement takeover, transfer, force, or stale Session
  recovery.
- This slice does not change Claim lifecycle semantics or release behavior.

## Consequences

- Operators and Agents can recover active Claim ids without reading the runnable
  projection or querying SQLite directly.
- Later takeover and transfer operations can build on this read-side surface
  without changing the stored Claim shape.

## Implementation Findings

- No schema change was required. `claim_runtime` already identifies the active
  Claim set, and immutable `claim` rows already preserve released occurrences.
