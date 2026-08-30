# ADR-0240: Phase 4EG Workspace Resource Association Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

Phase 4 continues to make the implemented local tool surface usable for focused
inspection. `workvcs resource workspace-association-list` already lists resource
associations for a workspace, but callers could not narrow the list to one
resource.

The engine already returns authoritative workspace-scoped associations. This
slice does not require a schema change, a new storage query, or resource
association lifecycle changes.

## Decision

`workvcs resource workspace-association-list` accepts:

- `--resource <RESOURCE_ID>`

The filter is an exact CLI-side match against `resource_id` after UUID
canonical parsing. When the resource id is well-formed but not associated with
the selected workspace, the command returns an empty association list.

## Consequences

Workspace/resource association inspection can now be narrowed without changing
the storage or engine surface.

This slice intentionally keeps the existing workspace-scoped engine list API as
the authoritative read boundary.
