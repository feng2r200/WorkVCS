# ADR-0182: Phase 4CA Claim Action Guard

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

The confirmed V1 runtime model requires terminal and structural Task mutations
to respect active Claims. Existing slices implemented Claim creation, release,
shared Claim sets, claim-next, next-work, and Claim observability, but there was
not yet a reusable Engine decision surface for tools to check whether one
Session may perform protected Task work.

## Decision

1. Core adds `ClaimGuardAction`, `ClaimGuardReason`, `ClaimGuardOptions`, and
   `ClaimGuardResult`.
2. `Engine::task_claim_guard` evaluates the requesting Session's active
   Workspace and Branch, verifies the target Task at the active Branch head, and
   reports whether the requested protected Task action is allowed.
3. For terminal and structural Task mutations, the guard allows:
   - no active Claim for that Task and Branch;
   - an active exclusive Claim owned by the requesting Session;
   - a unique active shared Claim owned by the requesting Session.
4. The guard rejects:
   - an active exclusive Claim owned by another Session;
   - a shared Claim set that does not include the requesting Session;
   - a non-unique shared Claim set.
5. Corrupt active Claim sets still fail closed as `ClaimInvalid`.
6. CLI adds `claim guard --session --task --action terminal-task` with
   `terminal-task` as the default and `structural-task` as the other supported
   action.

## Non-Goals

- This slice does not yet enforce the guard inside every historical mutation
  API.
- This slice does not implement transfer, force provenance, takeover, or stale
  Session recovery.
- This slice does not add a generalized policy DSL.

## Consequences

- Agents and scripts can ask the Engine for the Claim decision before attempting
  terminal or structural Task work.
- Later enforcement slices can reuse the same decision vocabulary instead of
  re-deriving Claim ownership rules independently.

## Implementation Findings

- The same decision rule applies to terminal and structural Task mutations in
  this slice. The action value is retained in the API so later enforcement can
  distinguish reporting or policy without changing the command shape.
