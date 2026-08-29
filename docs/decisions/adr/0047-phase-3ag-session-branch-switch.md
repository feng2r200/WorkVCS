# ADR-0047: Phase 3AG Session Branch Switch

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Runtime Coordination / Work Branch boundary.

## Context

Phase 3AF added O(1) Work Branch creation. After that, a local Agent could
create divergent Branches, but an active Session could still only remain on the
Branch selected at Session start. This left the CLI unable to continue work on
newly forked Branches without creating another Session.

## Decision

1. Phase 3AG adds `SessionSwitchOptions` and `SessionSwitchResult` to the
   Engine facade.
2. A Session switch atomically updates `session_runtime.active_workspace_id`
   and `session_runtime.active_branch_id`.
3. The target Branch must be active and belong to the target Workspace.
4. Switching clears existing Focus by default. A caller may set a new Focus
   entity if it is present at the target Branch head.
5. When the target Branch changes, active Claims owned by the Session on the
   previous Branch are released in the same transaction.
6. The switch writes a canonical `session.switched` Event with no ChangeSet.
7. The CLI exposes thin `session switch` over the Engine API.
8. This slice does not implement automatic `next`, claim takeover, Branch merge,
   session diff detail generation, cross-knowledge-space context, or Branch
   lifecycle transitions.

## Consequences

- A local Agent can now start work, create/fork a Work Branch, switch the
  Session to that Branch, and continue using runnable/claim workflows.
- Runtime switch state remains separate from immutable WorkState history.

## Implementation Findings

- Existing schema shape already supported atomic switch by updating
  `session_runtime` and using nullable-ChangeSet events; no schema migration was
  required.
