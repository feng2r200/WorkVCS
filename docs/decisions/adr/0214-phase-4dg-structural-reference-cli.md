# ADR-0214: Phase 4DG Structural Reference CLI

Status: Accepted

Date: 2026-08-30

## Context

Phase 3R implemented explicit Structural References between Goal, Plan, and
Task entities. Core exposes creation and historical listing through
`Engine::create_structural_reference` and `Engine::structural_references_at`,
but the CLI had no direct tool surface for creating or inspecting these
references.

## Decision

1. Add top-level `workvcs reference`.
2. Add `workvcs reference create STORE --branch BRANCH --head COMMIT --referrer
   ENTITY --target ENTITY`.
3. Add `workvcs reference list STORE` with exactly one target selector:
   `--branch` or `--commit`.
4. Parse ids through strongly typed canonical UUIDv7 parsers.
5. Reuse core Structural Reference validation, relation creation, CAS, and
   historical listing behavior.

## Non-Goals

- This slice does not add Structural Reference removal.
- This slice does not change valid endpoint kind pairs.
- This slice does not make Structural References affect primary containment or
  runnable-task scope.
- This slice does not add filtering or JSON output.

## Consequences

- Users can create and inspect explicit structural references from the CLI.
- Structural References remain separate from Task scheduling and primary
  containment relations.
- Historical listing is available without direct SQL access.
