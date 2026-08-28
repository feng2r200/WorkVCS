# ADR-0019: Phase 3E Session Runtime Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current Work request establishing the long-running
  implementation goal and authorizing continuous WorkVCS implementation from
  confirmed repository requirements.

## Context

Phase 3D made Verification Requirement and effective Acceptance Criterion
projection usable, but Runtime Coordination remained deferred. The confirmed
domain requires Sessions to be stable execution occurrences whose mutable
coordination state is separate from immutable Work State history.

Schema v0.1 already contains the Session-related tables needed for a narrow
foundation slice:

- `object_identity` supports `session` and `session_diff` object families;
- `session` stores the stable occurrence and metadata;
- `session_runtime` stores the current active Workspace, Branch, activity time,
  and runtime JSON;
- `session_context_workspace` and `session_context_knowledge_space` store
  current Context Set membership;
- `session_focus` and `session_focus_path` store structured Focus;
- `session_diff` stores final immutable Session provenance; and
- `event` records runtime provenance independently from ChangeSet and
  WorkStateCommit.

No schema migration is required for this slice.

## Decision

1. Phase 3E introduces only the Session Runtime Foundation in `workvcs-core`.
   It does not implement Claim, claim-next, Handoff, MergeRuntime,
   Resource/Evidence capture, Federation, Bundle, Checkpoint, schema migration,
   or business CLI commands.
2. The public surface remains Engine-owned and semantic. It must not expose
   SQLite handles, raw SQL, public generic Runtime CRUD, or CLI-to-SQL
   shortcuts.
3. A Session is created as a stable ObjectIdentity-backed occurrence with
   `object_kind = "session"` and one row in `session`.
4. Session metadata is canonical WorkVCS semantic JSON. Phase 3E accepts only a
   canonical object for metadata; non-object metadata is rejected before storage.
5. Session start creates one active `session_runtime` row, inserts the active
   Workspace into `session_context_workspace`, and writes a `session.started`
   Event in one `BEGIN IMMEDIATE` transaction.
6. Session start validates that the active Workspace exists, the active Branch
   exists, the Branch belongs to the Workspace, and the Branch is currently
   active.
7. Session runtime JSON is canonical WorkVCS semantic JSON. Phase 3E stores only
   the minimal lifecycle projection:

   ```json
   {
     "lifecycle_state": "active"
   }
   ```

8. Runtime Session operations do not create a ChangeSet, ChangeOperation,
   WorkStateCommit, or Branch HEAD movement unless a later semantic mutation is
   explicitly part of the operation. Phase 3E operations are pure Runtime
   Coordination operations.
9. Session read returns a deterministic runtime projection: Session id,
   lifecycle state, start time, last activity time when active, metadata, active
   Workspace, active Branch, context Workspaces, optional Focus, and final
   SessionDiff id when ended.
10. If `session_runtime` exists, the Session projects as `active` in Phase 3E.
    If `session_runtime` is absent and exactly one `session_diff` exists, the
    Session projects as `ended`. Missing both is invalid stored state.
11. `potentially_stale`, `starting`, and `ending` are confirmed lifecycle
    concepts but are not exposed as active API states in this slice. Abnormal
    exit detection and stale claim takeover remain deferred.
12. Focus is set or cleared through dedicated Engine semantic APIs. Focus set
    validates that the Session is active and that the focus Entity is present in
    the active Branch head WorkState.
13. A supplied Focus path is stored as ordered `session_focus_path` rows. Each
    path Entity must be present in the active Branch head WorkState. Each
    supplied incoming Relation must be present in the active Branch head
    WorkState.
14. Focus set replaces any existing Focus and path atomically, updates
    `last_activity_at_us`, and writes a `session.focus_set` Event.
15. Focus clear removes any existing Focus and path atomically, updates
    `last_activity_at_us`, and writes a `session.focus_cleared` Event. Clearing
    an absent Focus is idempotent.
16. Session end validates that the Session is active, creates an
    ObjectIdentity-backed `session_diff`, stores canonical object summary JSON,
    removes current Focus, Focus path, Context Set, and `session_runtime` rows,
    and writes a `session.ended` Event atomically.
17. Claim release on normal Session end is conceptually required by the domain,
    but Claim and ClaimRuntime are not implemented in Phase 3E. This slice must
    not expose Claim behavior or claim ownership decisions. The SessionEnd
    transaction may remove owned ClaimRuntime rows only when the schema contains
    such rows; no Claim API is introduced.
18. SessionEnd does not create a semantic Handoff. Handoff remains a separate
    versioned semantic operation.
19. Failed Session start, Focus set/clear, or Session end writes no partial
    authoritative runtime or provenance rows.
20. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3E does not add business Session commands.

## Consequences

- WorkVCS gains the first usable Runtime Coordination surface while preserving
  the separation between runtime state and versioned Work State history.
- Later Claim, claim-next, stale-session recovery, Handoff, and context resolver
  slices can build on stable Session occurrence, Focus, and SessionDiff records
  without redefining storage ownership.
- Runtime provenance becomes queryable through Events without implying an empty
  ChangeSet or Commit.

## Implementation findings

- `session_runtime` has no lifecycle or generation column. Because the confirmed
  physical schema also requires SessionEnd to remove current Session, Focus,
  Context Set, and owned Claim runtime rows, Phase 3E projects `ended` from the
  preserved `session` occurrence plus final `session_diff` and absence of an
  active `session_runtime` row.
- Session lifecycle transitions `starting -> active -> ending -> ended` are
  confirmed domain semantics, but Phase 3E has no durable intermediate runtime
  column for `starting` or `ending`. The implementation must treat start and end
  as atomic transactions whose externally visible states are active or ended.
- Focus validation can be implemented against the active Branch head WorkState
  using existing replay/query APIs. A richer context resolver is not required to
  make structured Focus storage valid.
- Context Knowledge Space membership exists in schema v0.1 but remains deferred
  in this slice because Knowledge Space runtime behavior is not implemented.
