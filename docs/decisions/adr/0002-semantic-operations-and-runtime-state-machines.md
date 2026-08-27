# ADR-0002: Semantic Operations and Runtime State Machines

- **Status:** Accepted
- **Accepted by:** explicit user confirmations for decisions 82–86, 88, and
  112–130; decision 87 was a procedural deferral, not a normative rule
- **Confirmation turns:** `3e66d1a4-50be-4941-b300-df7443efcba2`,
  `67d5e0f4-7156-4a8c-ae5e-acb65b4566e3`,
  `720c2494-7a6c-46e1-980f-9e03ca5f86bb`, and
  `5653e8a8-da77-4efe-840e-2ee7df9b2f70`

## Context

The baseline established atomic semantic mutation and separate Runtime
Coordination, but it did not yet fix the operation-specification method,
entity state transitions, cross-Workspace Handoff ownership, or the Session,
Claim, and merge-in-progress state machines.

## Decision

1. Architecture specifies Engine semantic operations before final CLI
   spelling. Each versioned operation defines inputs, structural/state/
   coordination preconditions, atomic state effects, generated relations,
   Events, claim and rationale requirements, commit behavior, and failure
   guarantees. Specifications are developed through complete work lifecycles,
   not by first enumerating commands.
2. A SessionDiff is cross-Workspace immutable Session provenance. A semantic
   Handoff is Versioned Work State: one Session may create zero or more
   Handoffs, and each Handoff belongs to exactly one Workspace and Work Branch
   with an optional Focus/context path.
3. Task non-terminal states are `pending`, `in_progress`, and explicit
   `blocked`; terminal states are `done`, `failed`, `cancelled`, and
   `superseded`. Dependency blocking is derived readiness, not Task status. An
   explicit atomic completion may move `pending` to `done` while retaining
   start/completion provenance. `done`, `failed`, and `cancelled` may be
   explicitly reopened or retried with rationale; `superseded` requires an
   operation that resolves its supersession relation.
4. Plan states are `active`, `completed`, `abandoned`, and `superseded`.
   Completion/abandonment may be explicitly reopened; supersession cannot be
   bypassed by an ordinary reopen. Goal states are `active`, `achieved`, and
   `abandoned`; achievement or abandonment may be explicitly reopened, while
   replacement is expressed by relation.
5. Assumption transitions are `unverified -> validated`, `unverified ->
   invalidated`, and `validated -> invalidated`. An invalidated Assumption is
   not ordinarily revalidated; a new/superseding Assumption represents changed
   conditions. Attempt terminal states are `succeeded`, `failed`, and
   `inconclusive`; a terminal Attempt is never reopened. Decision states are
   `active`, `superseded`, and `withdrawn`; re-adopting an old choice creates a
   new Decision that supersedes the current one.
6. Session runtime transitions are `starting -> active -> ending -> ended`,
   with `active <-> potentially_stale`; stale is not ended. Workspace and
   Branch switches are atomic. Workspace switch clears Focus unless a valid
   new Focus is supplied. Branch switch preserves Focus only when the entity
   and path remain valid. Session end atomically captures the final boundary,
   creates deterministic SessionDiff, releases default claims, clears Focus,
   and ends; Handoff remains separate.
7. The active Claim set for a Task/Branch is either empty, exactly one
   exclusive Claim, or one or more shared Claims. Exclusive and shared Claims
   cannot coexist. Mode change and takeover are explicit atomic operations
   with provenance; force takeover requires rationale. Claiming does not
   change Task status, and claim-next does not imply TaskStart.
8. MergeRuntime persists only `active`, `completed`, or `aborted`; resolving
   and ready conditions are derived from unresolved items. A target
   Workspace/Branch has at most one active merge. Resolutions remain
   provisional runtime state until `continue` atomically creates one complete
   ChangeSet and a two-parent WorkStateCommit. The merge captures base, source
   head, and target head, does not lock either Branch, and rejects continue if
   either head moved. Abort never changes target Work State and retains the
   attempt provenance.

## Consequences

- Coordination force cannot become a generic bypass for semantic invariants.
- State-machine errors can map to stable structural, state, and coordination
  categories with actionable recovery information.
- Merge abort is clean because provisional choices never became target Work
  State.
- Semantic Handoff remains compatible with the one-Workspace mutation
  invariant while Session provenance can span a Context Set.

## Deliberately not decided

- final CLI spelling and wire format;
- persistence identities for Claim replacement/mode change;
- a complete operation catalogue or full error-code enumeration;
- the exact source-head recomputation/rebase feature beyond V1;
- the exact AC waiver/exception operation.
