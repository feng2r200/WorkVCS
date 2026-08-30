# ADR-0209: Phase 4DB Workspace Resource Association List CLI

Status: Accepted

Date: 2026-08-30

## Context

Resource creation, binding, and Workspace association were already implemented,
and `resource show` rendered associations from a Resource-centric view. After
Workspace discovery was added to the CLI, users still lacked a Workspace-centric
way to inspect which Resources are associated with a Workspace.

## Decision

1. Add a read-only `Engine::workspace_resource_associations` facade.
2. Add `workvcs resource workspace-association-list STORE --workspace WORKSPACE`.
3. Require the target Workspace to exist before listing associations.
4. Return association metadata using the existing canonical JSON rendering.
5. Keep Resource association writes and Resource snapshots unchanged.

## Non-Goals

- This slice does not add association removal.
- This slice does not add Resource search or pagination.
- This slice does not change Resource binding semantics.
- This slice does not introduce adapters or external runtime behavior.

## Consequences

- Users can inspect Workspace-local Resource attachments without first knowing
  each Resource id.
- The Resource CLI now has both Resource-centric and Workspace-centric
  association reads.
- Missing Workspaces fail through the existing WorkspaceNotFound path.
