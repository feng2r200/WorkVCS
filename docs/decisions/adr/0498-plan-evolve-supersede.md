# ADR-0498: Plan Evolution Supersede Semantics

Status: Accepted / Implemented (current capability)
Date: 2026-09-10

## Context

P0-2b already supports atomic, idempotent in-place Plan evolution. A separate
supersede mode is needed when a Plan direction must be replaced while keeping
the prior Plan and its history explainable. Supersession is a semantic
transition, not an in-place edit and not a reason to silently move execution
state to a new Plan.

The existing P0 cutover ADR now treats `mode=supersede` as a current capability
after implementation and focused validation.

## Decision

The live manifest form is the existing Plan evolution entrypoint:

```text
workvcs plan evolve [OPTIONS] --manifest <PATH> <STORE|--cwd <PATH>>
```

The manifest selects `mode=supersede` and carries explicit target and
concurrency guards: the old Plan entity ID, expected old Plan entity version
ID, expected old Plan state digest, expected branch head commit ID, optional
expected state digest, and an idempotency key. The new Plan has structured
content and structured rationale. Its Goal parent is unique and explicit.

One successful supersede operation performs these semantic changes in one
transaction:

- the old Plan transitions from `active` to `superseded`;
- the new Plan is created as `active`;
- the same Goal retains the old `contains` relation and receives a new
  `contains` relation to the new Plan;
- a machine-readable `new_plan -> old_plan` `supersedes` relation is created;
- Plan constraints are handled explicitly by `carry_all` or `replace`; and
- no Tasks, Records, or Evidence are automatically migrated to the new Plan.

The operation is CAS-guarded by the expected IDs, versions, digests, and branch
head. It is idempotent for the same manifest identity, rejects conflicting
replays, and commits or exposes no partial semantic state. A failed guard or
validation leaves the old Plan and all related state unchanged; rollback is
therefore the transaction's atomic abort, not a compensating semantic edit.

The rationale is structured data and is part of the supersession provenance.
The operation must not encode supersession as `abandoned`, `completed`, or a
Decision-only annotation. Those states and annotations do not substitute for
the required Plan lifecycle transition and machine relation.

## Current capability boundary

`mode=supersede` is a current capability through `workvcs plan evolve`; its
manifest and guards are required as described above. Receipt issue/show/list
and consume are current P0-3a/P0-3b capabilities; receipt revoke and receipt
projection into `context`/`why` remain deferred. Read-only closeout inspect is
current as an independent mechanical projection.

## Non-goals

- No automatic migration of Tasks, Records, Evidence, Claims, Sessions, or
  Focus from the old Plan.
- No implicit constraint merge; the manifest must select `carry_all` or
  `replace`.
- No use of `abandoned`, `completed`, or a Decision-only note as a supersede
  substitute.
- No receipt-driven authorization-policy conclusion, receipt projection into
  `context`/`why`, remote, or repository-local registry behavior.

## Consequences

Superseded Plans remain queryable with their original containment and history,
while the new Plan has an explicit Goal parent and machine-readable ancestry.
Operators must deliberately rebuild or reference execution material for the
new Plan; this avoids silently changing the meaning or ownership of existing
Tasks, Records, and Evidence.
