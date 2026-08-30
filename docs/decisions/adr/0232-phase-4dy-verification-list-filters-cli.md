# ADR-0232: Phase 4DY Verification List Filters CLI

Status: Accepted

Date: 2026-08-30

## Context

Verification list can enumerate Verification snapshots at a Branch head or
historical Commit, but users need to inspect narrower subsets by result and
target. The existing list output already exposes target kind, target entity ID,
and result.

This slice follows the same read-side pattern as the prior list filter slices:
it narrows rendered snapshots without changing verification recording,
resource applicability, effective projection, or storage schema.

## Decision

1. Add `--target-kind KIND` to `workvcs verification list`.
2. Add `--target ENTITY_ID` to `workvcs verification list`.
3. Add `--result RESULT` to `workvcs verification list`.
4. Resolve the existing Branch/Commit target exactly as before.
5. Load Verification snapshots through the existing Engine facade and apply all
   supplied filters before rendering.
6. Keep the target-kind vocabulary limited to the confirmed Verification target
   families: `acceptance_criterion` and `verification_requirement`.

## Non-Goals

- This slice does not add new Verification target families.
- This slice does not change Verification recording, transition, or effective
  projection semantics.
- This slice does not filter by evidence, resource basis, method payload, or
  semantic dependency.
- This slice does not change SQLite schema, replay, or core snapshot APIs.

## Consequences

- CLI users can query passed, failed, or inconclusive Verification subsets
  directly.
- Verification review workflows can focus on one target family or one target
  entity without external post-processing.
- The command remains a thin read-only wrapper over existing Engine snapshots.
