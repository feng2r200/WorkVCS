# ADR-0046: Phase 3AF Branch Fork Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Branch / history invariants.

## Context

Phase 3 had enough semantic, verification, resource, session, claim, and
runnable tooling to operate on the initial Work Branch. The Engine could read a
Branch head, but it could not create another Work Branch from an existing
Branch head or historical WorkStateCommit. That left WorkVCS without the
minimal divergence operation required by the Branch model.

## Decision

1. Phase 3AF adds `BranchForkOptions` and `BranchForkResult` to the Engine
   facade.
2. A Branch fork is an O(1) ref operation: it inserts one active Branch row
   pointing at an existing WorkStateCommit.
3. Forking from a Branch resolves the current Branch head at transaction time;
   forking from a Commit uses that Commit's Workspace.
4. Branch fork does not create a ChangeSet, WorkStateCommit, Commit parent, or
   copied Entity/Relation state.
5. Branch fork records a canonical `branch.forked` Event with no ChangeSet so
   ref creation remains auditable.
6. The CLI exposes thin `branch head` and `branch fork` commands over the
   Engine API.
7. This slice does not implement merge, restore, Branch deletion, lifecycle
   transitions, session branch switching, or Branch-aware automatic next
   selection.

## Consequences

- WorkVCS can now create divergent Work Branches from current or historical
  state without copying WorkState.
- Existing `branch_head`, `history`, `show_at`, and runtime APIs can operate
  against forked Branches through the same public Engine boundary.

## Implementation Findings

- The schema already allowed event rows with a nullable `changeset_id`, so
  Branch creation could be made auditable without inventing an empty
  WorkStateCommit.
