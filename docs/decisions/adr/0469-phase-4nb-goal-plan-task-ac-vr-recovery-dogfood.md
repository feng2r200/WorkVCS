# ADR-0469: Phase 4NB Goal/Plan/Task and AC/VR Recovery Dogfood

Status: Accepted
Date: 2026-09-02

## Context

After ADR-0468, the Resource registration, observation, applicability, and
drift release gate had bounded V1-local pass evidence. The release gate matrix
still marked two related rows as blocking: Goal/Plan/Task ordering,
dependencies, and containment needed broader real-project repetition, and
AC/VR closure needed recovery and handoff-consumption evidence that included
Resource-backed stale/recovery behavior.

The missing evidence was not a new schema or command shape. WorkVCS already
exposes Goal, Plan, Task containment, scheduling relations, focused Handoff
consumption, Resource-backed verification, explicit batch Resource refresh,
and Task/Plan/Goal lifecycle commands. The release-maturity question was
whether those surfaces hold together in a realistic continuation and recovery
loop.

## Decision

Record Phase 4NB as a dogfood-only release-maturity evidence slice.

The slice uses a temporary clone of the pre-existing local `agent_soul`
repository and a WorkVCS Store under `/tmp`. It creates one Goal, one Plan, and
three contained Tasks. The Tasks are connected by two `depends_on` scheduling
relations and two `ordered_before` relations. The run proves the dependent
Tasks are blocked before their prerequisites close, that brief context exposes
the blocked dependency, and that a Task cannot transition to `done` before its
AC/VR-backed verification is recorded.

The same Store then continues through a focused Handoff. A source Session
focuses the dependent Task, ends with a SessionDiff, and authors a focused
Handoff. A separate continuation Session consumes the Handoff, claims the
focused Task, records a baseline Resource-backed Verification, mutates only the
temporary clone, explicitly refreshes Resource applicability, observes
`stale` / `resource_drift`, records recovery Verification evidence, and closes
the dependent Task, final Task, Plan, Goal, Session, Claim, Store integrity,
and doctor checks.

The Goal/Plan/Task and AC/VR release gates can move to `Pass` for the bounded
V1-local scope. The overall release decision remains false because Store
maintenance/portability, context/why breadth, operator recovery maturity,
larger workload evidence, and candidate release operation gates still block
V1 release maturity.

## Non-Goals

- No Rust code, schema, or command-shape change.
- No release, release candidate, tag, Push, or deployment.
- No direct mutation of the original `agent_soul` repository.
- No remote/cloud Handoff, cross-Store synchronization, distributed
  collaboration, automatic takeover, or Agent orchestration claim.
- No daemon, watcher, automatic polling, implicit refresh, or scheduler.
- No V2 scope expansion, including transcript parsing, LLM semantic extraction,
  embeddings/vector search, semantic merge, automatic knowledge distillation,
  or GUI/TUI requirements.
- No broad Store maintenance, performance, context traversal, or release-ready
  claim.

## Evidence

- Contract inspection:
  `/tmp/workvcs-4nb-contract-inspection-20260901T181900Z/summary.txt`.
- T-002 dogfood evidence:
  `/tmp/workvcs-4nb-goal-plan-task-ac-vr-recovery-dogfood-20260901T184100Z/summary-t002.txt`.
- T-003 dogfood evidence:
  `/tmp/workvcs-4nb-goal-plan-task-ac-vr-recovery-dogfood-20260901T184100Z/summary-t003.txt`.
- Dogfood Store:
  `/tmp/workvcs-4nb-goal-plan-task-ac-vr-recovery-dogfood-20260901T184100Z/work/workvcs.sqlite`.
- Temporary clone:
  `/tmp/workvcs-4nb-goal-plan-task-ac-vr-recovery-dogfood-20260901T184100Z/work/agent_soul-4nb-clone`.
- Provenance document:
  `docs/provenance/phase-4nb-goal-plan-task-ac-vr-recovery-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

WorkVCS now has current dogfood evidence that Goal/Plan/Task dependency
ordering and containment can govern a real continuation workflow through
closeout, and that AC/VR-backed evidence closure remains understandable during
Handoff consumption and Resource-backed stale recovery.

The release matrix remains non-ready. The next highest-value work should move
to another blocking gate: broader Store maintenance/portability, context and
`why` explanation breadth, operator recovery maturity, larger workload
evidence, or the final candidate-release operation after all functional gates
pass and explicit release authorization exists.
