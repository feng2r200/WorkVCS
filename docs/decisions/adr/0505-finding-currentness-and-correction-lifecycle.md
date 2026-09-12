# ADR-0505: Finding Currentness And Correction Lifecycle

Status: Accepted
Date: 2026-09-12

## Context

A fresh installed-binary Recall probe showed a correcting Finding and the
inaccurate Finding it corrected as two active current facts. Ordering the newer
Record first reduced one symptom but could not express that the earlier
observation was no longer valid. This makes bounded recovery unsafe: another
Agent can still act on disproved cognition, and retrospective output cannot
distinguish current truth from preserved history.

Attempt already has a running-to-terminal state machine, but the CLI help said
that `attempt-status` used `abandoned` even though the implemented terminal
state is `inconclusive`, and `record attempt` did not make clear that it starts
a running Attempt. This encouraged Records to remain accidentally open.

## Decision

1. Finding has `active`, `superseded`, and `invalidated` states.
2. `active -> superseded` requires an active replacement Finding. One atomic
   operation updates the prior Finding and creates the canonical
   `replacement -> prior` `supersedes` relation.
3. `active -> invalidated` requires an active Finding that disproves the
   target. One atomic operation updates the target Finding and creates the
   canonical `cause -> target` `invalidates` relation.
4. Both endpoints belong to the same Workspace, the source and target differ,
   the target version is explicitly guarded, and Branch-head compare-and-swap
   protects the complete Entity-plus-Relation transition.
5. Terminal Findings are never reactivated or corrected again. Changed
   conditions or a later correction create another Finding and a new explicit
   evolution edge.
6. Brief Recall includes only current Record states. Handoff Recall keeps
   current Records plus terminal Attempts that prevent repeated work.
   Retrospective Recall retains terminal cognition but orders current Records
   and Knowledge before historical or terminal entries and labels their
   temporal scope.
7. A validated Assumption remains current for Recall purposes; currentness is
   not limited to the literal `active` status name.
8. CLI help states that `record attempt` starts a running Attempt and that
   `attempt-status` finishes it as `succeeded`, `failed`, or `inconclusive`.

## Consequences

- Corrected or disproved Findings no longer masquerade as current project
  truth in ordinary bounded recovery.
- Historical statements, their status transitions, and their typed causal
  relations remain available for `record show/list`, `why`, history, bundle,
  merge, and retrospective use.
- Finding correction is not implemented as two independently failing writes;
  callers receive one committed Work-State result or no change.
- No new table or schema migration is required because Record status and typed
  relations already use versioned Entity and Relation state.

## Non-goals

- Automatically inferring that two natural-language Findings conflict.
- Treating recency alone as supersession or invalidation.
- Adding closure lifecycles for Question, Risk, or Handoff in this slice.
- Reopening terminal Attempts or terminal Findings.
