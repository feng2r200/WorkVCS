# ADR-0421: Phase 4LF Handoff Consumption Dogfood

Status: Accepted
Date: 2026-09-01

## Context

Phase 4LE corrected a recovery-policy gap and dogfooded a full local
implementation closeout, but the V1 readiness ledger still identified a
handoff-continuation gap: WorkVCS had authored and shown focused Handoffs, but
had not used one as the continuation entrypoint for a later Session.

The 4LF dogfood run proved that continuation is possible with existing commands:
`handoff show` exposes `focus_entity_id`, `session focus-set` can apply that
focus to a new active Session, `context` renders the focused current Task and
relevant Handoff, and `next` can claim the focused continuation Task.

The same run exposed a real operator-friction gap: a continuation Agent must
manually copy the focus id from `handoff show` into `session focus-set`.

## Decision

Add a thin CLI composition command:

```text
workvcs handoff consume STORE --commit COMMIT --handoff HANDOFF --session SESSION
```

The command:

1. resolves the Handoff record at a branch head or explicit commit;
2. requires the record to be `Record(kind=handoff)`;
3. requires a recognized focused Handoff scope with a non-null
   `focus_entity_id`;
4. validates the referenced SessionDiff when present;
5. calls the existing Engine-owned Session focus update API for the target
   continuation Session;
6. renders both the Handoff scope and the focus update result; and
7. supports expectations for source Session, SessionDiff, focus, continuation
   Session, and continuation Session lifecycle state.

This is a CLI composition over existing Engine operations. It does not add a new
schema table, a new WorkState entity, an automatic Agent handoff daemon, or a
new `why` relation.

## Non-Goals

- No automatic Session creation from a Handoff.
- No implicit claim, Task transition, verification, or recovery action.
- No schema migration or new Handoff runtime table.
- No `why` relation for Handoff focus until the relation semantics are
  explicitly decided.
- No remote/cloud handoff or Agent orchestration.

## Consequences

The continuation loop now has a first-class CLI entrypoint:

1. author or show a focused Handoff;
2. start a continuation Session;
3. consume the Handoff into that Session's focus;
4. inspect Context and `next`; and
5. claim or continue work explicitly.

This addresses one concrete dogfood friction point without broadening the V1
surface. The remaining related gap is explanation quality: current `why` output
does not expose the Handoff focus link as a relation, so the focus connection is
visible through `handoff show` and `context`, not through `why`.
