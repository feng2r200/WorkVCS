# ADR-0507: Question And Risk Currentness Lifecycle

Status: Accepted
Date: 2026-09-12

## Context

A bounded multi-Agent Recall pilot recovered the active Goal, Plan, Task,
Session, Claim, and priority order, but a consumer also found an old active
Risk stating that Bundle export did not carry raw Evidence. Bundle v2 had
already repaired and verified that problem. The record remained mechanically
current because Risk had no terminal lifecycle. The same pilot was intended to
answer a durable token-attribution Question, but Question likewise had no way
to stop being a current unknown.

ADR-0505 deliberately left Question and Risk closure out of its Finding-focused
slice. The bridge pilot provides the missing evidence that this is now a
currentness defect rather than an optional taxonomy expansion.

## Decision

1. Question uses `active -> answered | deferred | withdrawn`.
2. Risk uses `active -> mitigated | invalidated | withdrawn`.
3. All transitions require a guarded Branch head, the current Record version,
   and non-empty rationale. Terminal Question and Risk states cannot reopen;
   renewed conditions create a new Record.
4. CLI exposes `record question-status` and `record risk-status` with only the
   states valid for that Record kind.
5. Brief and Handoff Recall treat the new terminal states as historical rather
   than current. Retrospective retains them with their terminal status.
6. This slice adds no tables, transcript inference, automatic closure,
   replacement relation taxonomy, or compatibility layer.

## Consequences

- Resolved unknowns and exposures no longer compete with current project truth
  in bounded multi-Agent recovery.
- The original statement, terminal rationale, Entity versions, Events, and
  history remain inspectable.
- The Store schema does not migrate because Record status is versioned Entity
  state. Older binaries do not understand the new status strings and must not
  be used after these states are written; the globally installed binary and
  Skill therefore need to move together.

## Non-goals

- Automatically deciding that a Question is answered or a Risk is mitigated.
- Inferring closure from a newer Finding or from text similarity.
- Adding Question/Risk supersession commands or new causal relation types in
  this slice.
- Expanding bounded Recall so an arbitrary terminal Task is always present.
- Requiring a Handoff for every delegation or Session end.
