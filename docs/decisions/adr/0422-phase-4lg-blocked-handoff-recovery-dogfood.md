# ADR-0422: Phase 4LG Blocked Handoff Recovery Dogfood

Status: Accepted
Date: 2026-09-01

## Context

Phase 4LE made forced Claim takeover require an explicit
`potentially_stale` previous owning Session. Phase 4LF then made focused
Handoffs consumable by a continuation Session. The next V1 readiness gap was to
prove that those pieces work together in a real blocked continuation path:

1. a focused Handoff points to a continuation Task;
2. another active Session already owns an exclusive Claim on that Task;
3. the continuation Session consumes the Handoff but cannot safely proceed; and
4. recovery requires mark-stale evidence before takeover.

## Decision

Use the existing command surface as the V1 recovery contract for blocked focused
Handoffs:

```text
workvcs handoff consume STORE --commit COMMIT --handoff HANDOFF --session CONTINUATION
workvcs claim guard STORE --session CONTINUATION --task TASK
workvcs session mark-stale STORE --session PREVIOUS_OWNER --rationale TEXT
workvcs claim takeover STORE --session CONTINUATION --claim CLAIM --force --rationale TEXT
```

No new implementation is required for this slice. The dogfood evidence proves
the current semantics:

1. `handoff consume` restores the focused continuation Task to the continuation
   Session;
2. `claim guard` reports `exclusive_claim_owned_by_other_session`;
3. `next` does not select the blocked focused Task;
4. forced takeover is only used after the previous owner is marked
   `potentially_stale`; and
5. takeover records `previous_session_lifecycle_state=potentially_stale`, after
   which guard reports `owned_exclusive_claim` for the continuation Session.

## Non-Goals

- No automatic stale detection.
- No automatic takeover during Handoff consumption.
- No implicit claim or Task transition when consuming a Handoff.
- No schema migration or Claim runtime redesign.
- No Agent orchestration, remote lock negotiation, or distributed recovery.

## Consequences

WorkVCS now has dogfood evidence for the V1 handoff-recovery loop without adding
new behavior beyond Phase 4LF. The remaining operator friction is explicit ID
capture: operators still need the previous owning Session id and Claim id before
mark-stale/takeover. Further CLI changes should only target repeated dogfood
friction, not display-only convenience.
