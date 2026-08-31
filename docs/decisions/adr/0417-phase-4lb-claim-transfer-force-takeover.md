# ADR-0417: Phase 4LB Claim Transfer and Force Takeover

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger identifies Claim recovery as the next dogfood-biased
gap. Current WorkVCS can create, release, list, and guard active Claims, and it
can atomically claim the next runnable Task. It cannot yet recover a blocked
Task when the current claimant needs to hand work to another Session or when an
operator must override an active Claim with explicit provenance.

The confirmed model says each Claim ownership/mode period is a stable
occurrence. Release, mode change, and takeover end the old occurrence and create
another instead of rewriting the prior Claim.

The current Session runtime only implements `active` and `ended`. Because
`potentially_stale` is not yet implemented, this slice must not pretend to prove
stale takeover.

## Decision

1. Add explicit Claim replacement operations over the existing schema:
   `claim transfer` and forced `claim takeover`.
2. Transfer requires an active source Session that owns the active Claim and an
   active target Session on the same Workspace and Branch.
3. Forced takeover requires an active taking Session on the same Workspace and
   Branch as the active Claim plus a non-empty rationale.
4. Both operations delete the replaced Claim's current `claim_runtime` row,
   insert a new ObjectIdentity-backed Claim occurrence, insert a new
   `claim_runtime` row, update the acting Session activity, and write an
   immutable Event payload naming the prior Claim, prior Session, prior mode,
   prior last activity, replacement Claim, and reason.
5. Existing Claim active-set rules remain the enforcement mechanism after the
   replaced Claim runtime row is removed inside the same transaction.

## Non-Goals

- No `potentially_stale` Session lifecycle implementation.
- No automatic stale detection or timeout policy.
- No structural Task mutation `--force` bypass.
- No schema migration, new Claim history table, or typed replacement relation.
- No remote coordination or agent orchestration.

## Consequences

Agents can now unblock a claimed Task by explicit transfer or by a forced
takeover with rationale, while preserving the old Claim occurrence and recording
why the replacement happened.

Stale takeover remains a V1 gap, but it is now narrower: it needs a real stale
Session state and policy instead of a new Claim replacement storage shape.
