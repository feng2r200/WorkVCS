# ADR-0199: Phase 4CR Resource Observation Show CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Resource Observation rows are already created through the CLI and readable
through the Engine facade. The CLI did not expose direct Observation readback,
which made Resource-backed Verification workflows depend on create output or
direct SQL for later inspection.

## Decision

1. Add `resource observation-show`.
2. Resolve by Resource Observation id and delegate to the existing Engine
   read-only API.
3. Render adapter fields, fingerprint, captured time, summary JSON, optional
   detail-content fields, and optional source session id.
4. Keep Resource Observation creation semantics unchanged.

## Non-Goals

- This slice does not add Resource Observation listing.
- This slice does not add new detail-content CLI inputs.
- This slice does not change Resource Basis validation.
- This slice does not introduce adapter-specific behavior.

## Consequences

- CLI users can inspect persisted Resource Observations without direct SQL.
- Resource-backed Verification setup has a complete create/show loop for
  Observation identity and fingerprint fields.

## Implementation Findings

- None.
