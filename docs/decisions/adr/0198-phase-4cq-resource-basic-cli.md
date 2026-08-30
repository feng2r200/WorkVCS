# ADR-0198: Phase 4CQ Resource Basic CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Resource objects, Resource bindings, and Workspace-Resource associations already
exist in the core Engine. CLI workflows could create Resources and observations,
but could not show Resource state or establish Resource binding/Workspace
association rows directly.

## Decision

1. Add `resource show`.
2. Add `resource bind` with adapter kind, locator, and canonical object binding
   config.
3. Add `resource associate-workspace` with Workspace id, Resource id, and
   canonical object metadata.
4. Render Resource snapshots with binding presence, binding fields, and
   Workspace association fields.
5. Continue to use the existing canonical semantic JSON object parser for
   binding config and association metadata.

## Non-Goals

- This slice does not add Resource listing.
- This slice does not change Resource Observation creation.
- This slice does not add adapter-specific validation.
- This slice does not introduce Resource lifecycle state.

## Consequences

- CLI users can inspect and configure Resource identity rows without direct SQL.
- Resource-backed Verification workflows can be prepared through CLI commands.

## Implementation Findings

- None.
