# ADR-0049: Phase 3AI Branch List

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Work Branch / Engine facade boundary.

## Context

After Phase 3AF and 3AG, WorkVCS could create Branches and switch Sessions
between Branches, but the Engine and CLI did not expose a way to discover the
Branches in a Workspace. That made the command shell less usable for ongoing
multi-Branch work.

## Decision

1. Phase 3AI adds `Engine::list_branches(workspace_id)`.
2. The operation returns current `BranchHead` snapshots ordered by Branch name
   and Branch id.
3. Listing validates the Workspace exists and reuses the existing Branch head
   projection for each row.
4. The CLI exposes thin `branch list STORE --workspace <id>` output in the
   existing line-oriented `key=value` style.
5. This slice does not implement Branch lifecycle transitions, Branch deletion,
   merge status, stale projection repair, or pagination.

## Consequences

- A local Agent can now inspect available Work Branches before selecting a
  Branch head, switching a Session, or running history/runnable queries.
- Branch listing remains read-only and does not create WorkState history or
  runtime rows.

## Implementation Findings

- No schema change was required; the existing `branch` table plus `BranchHead`
  query path already contained the required data.
