# ADR-0022: Phase 3H Task Scheduling Relation Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed relation/readiness requirements.

## Context

Phase 3G added read-only Runnable Task Projection, but it explicitly deferred
dependency readiness and manual order because the Rust semantic API had no
explicit creation/read boundary for Task scheduling relations.

Confirmed relation semantics define:

```text
depends_on      dependent -> prerequisite
ordered_before  earlier -> later
```

They are independent dimensions. `depends_on` affects readiness and must remain
acyclic. `ordered_before` is preferred sibling order and must not imply a
dependency. Priority remains separate from manual order, and the Phase 3G
implementation finding still applies: the Rust contract does not yet define
integer priority direction.

The v0.1 schema already has Relation identity/version tables,
`relation_membership_change`, relation ChangeOperations, and normal replay
support for relation creations.

## Decision

1. Phase 3H introduces only explicit Task-to-Task scheduling relation
   foundation behavior in `workvcs-core`.
2. The implemented relation types are `depends_on` and `ordered_before`.
   `contains`, `references`, `related_to`, evolution, epistemic, and broader
   Verification relation APIs remain out of scope.
3. The public surface remains Engine-owned and semantic. It must not expose
   SQLite handles, raw SQL, public generic projection CRUD, or CLI-to-SQL
   shortcuts.
4. Relation creation validates the target Branch exists, belongs to one
   Workspace, is active, and still points at the caller-provided expected head
   when the write is installed.
5. Source and target endpoints must both be current Task entities at the
   expected head and belong to the Branch Workspace.
6. Self-edges are rejected for both implemented relation types.
7. `depends_on` creation rejects cycles in the current dependency graph.
8. `ordered_before` creation is independent from `depends_on`; it does not make
   either endpoint runnable or blocked and does not interpret priority.
9. A successful creation writes one normal WorkStateCommit with a relation
   ChangeOperation, relation_membership_change, Relation identity, Relation
   Version, ChangeSet, Event, primary parent, and Branch HEAD movement
   atomically.
10. RelationVersion state is canonical empty object metadata for this
    foundation slice. The relation logical key carries type/source/target and
    the empty discriminator.
11. Re-adding a relation already current at the expected WorkState is rejected
    as a no-op.
12. Reusing a historical logical relation identity after removal is deferred
    because relation removal/update is not implemented in Phase 3H.
13. Phase 3H adds read/query behavior for Task scheduling relations at a commit
    so later readiness and `next` slices consume semantic relation facts
    without raw SQL.
14. Existing verification projection must ignore non-`verifies` relations while
    retaining strict validation for malformed `verifies` relations.
15. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3H does not add business relation, runnable, or `next` commands.

## Consequences

- Later readiness projection can reason over stored Task dependencies without
  introducing a second relation authority.
- Manual order facts become visible without implying dependencies or priority
  ordering.
- The complete `next` resolver remains incomplete until Plan path,
  executable-descendant traversal, dependency readiness rules, manual order
  selection, and the final tie-breaker are implemented or frozen.

## Implementation Findings

- `workctl plan admit apply` requires a bootstrap READY receipt that is not
  present in the new Phase 3H worktree. The implementation therefore records
  this Plan/ADR in the repository and uses conversation-level task tracking,
  independent review, and full validation as the fallback governance evidence.
- The confirmed relation model says a removed and later re-added same logical
  relation reuses its identity. Phase 3H has no relation removal/update, so
  identity reuse after removal is explicitly deferred.
- `contains` is left out of this slice because primary containment touches Plan
  path/executable descendants and mixed Plan/Task containers, while Plan
  entities are not implemented yet.
- Priority direction remains unfixed for the Rust implementation contract and
  is not interpreted by Task scheduling relation creation.
- Existing verification projection code assumed every current relation was a
  `verifies` relation. Phase 3H fixes that assumption by filtering non-`verifies`
  relations in verification-specific readers and adds a mixed relation
  regression test.
