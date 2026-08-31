# ADR-0419: Phase 4LD Explicit Potentially Stale Session State

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger kept stale Claim takeover open because the runtime only
distinguished active and ended Sessions. Phase 4LB added explicit Claim
transfer and forced takeover, but deliberately avoided fake stale semantics.

The schema already stores Session runtime lifecycle state in
`session_runtime.runtime_json` and does not constrain it to only `active`.
Therefore the next recovery step can add the missing Session state without a
schema migration.

## Decision

Add an explicit `potentially_stale` Session lifecycle state and expose it as:

```text
workvcs session mark-stale STORE --session SESSION_ID --rationale TEXT
```

The operation:

1. requires the target Session to be active;
2. requires a non-empty rationale;
3. rewrites only `session_runtime.runtime_json` to
   `{"lifecycle_state":"potentially_stale"}`;
4. preserves the active Workspace, Branch, context Workspace set, focus, and
   active Claims for recovery inspection;
5. records a `session.potentially_stale` Event with the prior lifecycle state,
   prior last activity, active target, and rationale; and
6. does not create a WorkStateCommit, ChangeSet, SessionDiff, or schema
   migration.

Active-only operations continue to require `SessionLifecycleState::Active`.
`session end` now accepts a potentially stale runtime Session so an operator can
close it, create the SessionDiff, and release active Claims through the existing
end-session cleanup path.

## Non-Goals

- No automatic stale detection, timeout policy, background heartbeat, or daemon.
- No stale-gated Claim takeover policy.
- No change to forced Claim takeover behavior.
- No schema migration or new Session lifecycle table.
- No remote coordination or Agent orchestration.

## Consequences

WorkVCS now has the missing runtime state needed before designing stale Claim
takeover. Recovery can distinguish "still active" from "possibly abandoned"
without deleting the Session runtime or releasing Claims prematurely.
