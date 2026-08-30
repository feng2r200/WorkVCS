# ADR-0190: Phase 4CI Goal And Plan Show CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Goal and Plan creation and lifecycle transition commands are now available from
the CLI, but users still need a read-only way to inspect the resulting semantic
state without falling back to direct Engine calls or broad `show-at` output.

## Decision

1. Add `goal show` as a thin wrapper around `Engine::goal_at`.
2. Add `plan show` as a thin wrapper around `Engine::plan_at`.
3. Allow exactly one query target for each command: `--branch` for the current
   branch head or `--commit` for historical state.
4. Render snapshot outputs as stable key/value lines covering commit id, entity
   id, entity version id, state digest, status, and semantic fields.
5. Render free-text fields and arrays as canonical JSON values so CLI output
   remains unambiguous for scripts.

## Non-Goals

- This slice does not add Goal or Plan list commands.
- This slice does not change Goal or Plan lifecycle rules.
- This slice does not add new Engine query capabilities.
- This slice does not alter Task, Verification, Merge, Federation, Runtime, or
  Store behavior.

## Consequences

- CLI users can inspect Goal and Plan state at a branch head or at a historical
  commit.
- Lifecycle CLI workflows now have direct read-back verification through the
  same thin command shell.

## Implementation Findings

- None.
