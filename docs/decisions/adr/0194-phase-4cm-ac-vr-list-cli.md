# ADR-0194: Phase 4CM Acceptance Criterion And Verification Requirement List CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Acceptance Criteria and Verification Requirements now have direct CLI show
commands. CLI workflows also need compact inventories of those objects at a
branch head or historical commit, especially after a VR create operation updates
the owning Acceptance Criterion version.

## Decision

1. Add read-only `Engine::acceptance_criteria_at` and
   `Engine::verification_requirements_at` facade methods.
2. Implement both APIs by replaying WorkState at the target commit and loading
   the AC/VR entity versions referenced by that WorkState.
3. Sort AC snapshots by task id, local key, and AC entity id; sort VR snapshots
   by owning AC id, local key, and VR entity id.
4. Add `ac list` and `vr list` CLI commands with the same `--branch` or
   `--commit` target selector used by show commands.
5. Render list outputs with stable `criterion.N.*` and `requirement.N.*`
   key/value fields.

## Non-Goals

- This slice does not add filtering or pagination.
- This slice does not revise AC or VR state.
- This slice does not alter effective verification projection semantics.
- This slice does not materialize a new projection table.

## Consequences

- CLI users can enumerate acceptance and verification requirements directly.
- Historical lists reflect the WorkState-visible entity versions at the target
  commit.

## Implementation Findings

- None.
