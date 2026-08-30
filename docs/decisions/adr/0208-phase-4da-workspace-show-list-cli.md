# ADR-0208: Phase 4DA Workspace Show/List CLI

Status: Accepted

Date: 2026-08-30

## Context

The Engine facade already supported creating a Workspace and loading a single
Workspace by id. Later CLI surfaces such as branch, session, resource, and
runtime commands require a Workspace id, but the CLI had no direct way to show
or enumerate Workspaces in a store.

## Decision

1. Add `Engine::workspaces` as a read-only Workspace list facade.
2. Add `workvcs workspace show STORE --workspace WORKSPACE`.
3. Add `workvcs workspace list STORE`.
4. Reuse the existing Genesis shape validation for every listed Workspace.
5. Render Workspace display name and creation timestamp in Workspace output.

## Non-Goals

- This slice does not add Workspace filters, pagination, or search.
- This slice does not change Workspace creation semantics.
- This slice does not change the SQLite schema.
- This slice does not introduce runtime, merge, bundle, or federation behavior.

## Consequences

- Users can discover Workspace ids from the CLI before running branch, session,
  and resource workflows.
- Workspace list reads remain inside the Store/Engine boundary.
- Corrupt Workspace rows continue to fail through Genesis validation instead of
  being silently rendered.
