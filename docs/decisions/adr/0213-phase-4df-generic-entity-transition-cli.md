# ADR-0213: Phase 4DF Generic Entity Transition CLI

Status: Accepted

Date: 2026-08-30

## Context

Phase 2 implemented the generic Entity transition path: canonical entity state
storage, EntityVersion digests, WorkState membership changes, branch-head CAS,
changesets, operations, commits, parents, and events. Later CLI slices exposed
semantic wrappers such as Task, Goal, Plan, Knowledge, Record, Acceptance
Criterion, and Verification, but the base Entity transition operation still had
no direct tool surface.

## Decision

1. Add top-level `workvcs entity`.
2. Add `workvcs entity create STORE --branch BRANCH --head COMMIT --kind KIND
   --state-json JSON`.
3. Add `workvcs entity update STORE --branch BRANCH --head COMMIT --entity
   ENTITY --entity-version VERSION --state-json JSON`.
4. Accept optional `--rationale-json` and require it to be canonical semantic
   JSON object input.
5. Reuse `EntityTransitionOptions` and `Engine::commit_entity_transition`; the
   CLI does not duplicate CAS, digest, changeset, or WorkState logic.

## Non-Goals

- This slice does not add relation transitions.
- This slice does not add semantic validation for domain-specific entity kinds.
- This slice does not bypass Task, Goal, Plan, Knowledge, Record, Acceptance
  Criterion, or Verification wrappers.
- This slice does not add deletion or tombstone behavior.

## Consequences

- Operators can exercise the confirmed Phase 2 Entity transition path directly.
- Generic entity commits remain subject to the same branch-head CAS and
  canonical digest rules as semantic wrappers.
- The CLI can now support lower-level validation and migration workflows without
  direct SQL access.
