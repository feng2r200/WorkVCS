# ADR-0193: Phase 4CL Acceptance Criterion And Verification Requirement Show CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Acceptance Criteria and Verification Requirements are central to the confirmed
verification model. The CLI could create both and query effective AC status, but
it lacked direct snapshot inspection commands for the underlying semantic
objects.

## Decision

1. Add `ac show` as a thin wrapper around `Engine::acceptance_criterion_at`.
2. Add `vr show` as a thin wrapper around `Engine::verification_requirement_at`.
3. Allow exactly one query target for each command: `--branch` for current
   branch head or `--commit` for historical state.
4. Render AC snapshots with task id, local key, AC entity/version ids, state
   digest, classification, statement JSON, and attached VR refs.
5. Render VR snapshots with owning AC id, local key, VR entity/version ids,
   state digest, and statement JSON.

## Non-Goals

- This slice does not add AC or VR list commands.
- This slice does not revise AC or VR state.
- This slice does not alter effective verification projection semantics.
- This slice does not change Task lifecycle, claim, Merge, Federation, Runtime,
  or Store mutation behavior.

## Consequences

- CLI users can inspect the semantic objects that define acceptance and
  verification requirements directly.
- Existing AC status and verification workflows gain precise read-back commands
  without expanding the storage model.

## Implementation Findings

- None.
