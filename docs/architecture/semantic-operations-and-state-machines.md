# Semantic Operations and State Machines

This document defines the confirmed operation-design method and V1 semantic
and runtime state machines. It realizes
[ADR-0002](../decisions/adr/0002-semantic-operations-and-runtime-state-machines.md).

## Operation contract

Final CLI spelling is downstream of the Engine semantic contract. Every
versioned semantic operation specifies:

```text
Inputs
Structural Preconditions
State Preconditions
Coordination Preconditions
Atomic State Effects
Generated Relations
Provenance Events
Claim Requirements
Rationale Requirements
Commit Behavior
Failure Guarantees
```

Specifications are validated through complete work lifecycles—for example,
Task execution through Attempt, Finding, Assumption invalidation, Decision,
supersession, replacement, Verification, and completion—rather than by first
inventing a command tree.

A versioned operation creates one atomic ChangeSet and exactly one
WorkStateCommit. A pure Runtime Coordination operation atomically changes
runtime state, emits provenance, and creates no WorkStateCommit. A command
execution that returns a failing test result can still be a successfully
recorded failed Verification; persistence failure is an operation failure.

Errors identify the failed precondition category and provide the relevant
state and recovery action. The complete code vocabulary remains an
implementation specification.

## Semantic entity state machines

### Task

```text
non-terminal: pending | in_progress | blocked
terminal:     done | failed | cancelled | superseded
```

`blocked` means an explicit execution blocker that dependency edges cannot
express. Dependency blocking remains derived readiness. Standard transitions
include start, explicit block/unblock, completion, failure, cancellation, and
supersession. An explicit atomic completion may take `pending` directly to
`done` while recording both start and completion provenance.

`done`, `failed`, and `cancelled` may return to non-terminal state only through
an explicit reopen/retry operation with rationale. `superseded` cannot use an
ordinary reopen because the supersession relation must also be resolved.

### Plan and Goal

```text
Plan: active | completed | abandoned | superseded
Goal: active | achieved | abandoned
```

Plan completion and Goal achievement remain explicit even when derived progress
is complete. Completed or abandoned Plans and achieved or abandoned Goals may
be explicitly reopened with rationale. A superseded Plan requires
supersession-aware resolution. Goal replacement is expressed through a
relation rather than adding a mandatory Goal `superseded` state.

### Assumption, Attempt, and Decision

```text
Assumption:
  unverified -> validated
  unverified -> invalidated
  validated  -> invalidated

Attempt:
  running -> succeeded | failed | inconclusive

Decision:
  active -> superseded | withdrawn
```

An invalidated Assumption is not ordinarily revalidated; changed conditions
produce a new or superseding Assumption. A terminal Attempt is never reopened;
a later try is a new Attempt. An old Decision is never reactivated. Re-adopting
an earlier choice creates a new Decision that supersedes the currently active
Decision. Explicit Decision supersession does not require identical
`scope+subject`, and matching `scope+subject` does not itself imply
supersession.

Major transitions carry rationale text, a causal Entity reference, or both as
defined by their semantic contract. Coordination `force` cannot bypass a
mandatory AC gate; any waiver is a distinct semantic concept.

## Handoff and Session continuity

SessionDiff is immutable Session-global provenance and may summarize changes
across every Workspace visited by the Session. Handoff is not global runtime
state:

- a Session may create zero or more Handoff Records;
- each Handoff belongs to exactly one Workspace and Work Branch;
- a Handoff may bind a Focus/context path;
- Handoff creation is a separate versioned semantic operation;
- Session end may recommend, but never fabricates or requires, a Handoff.

## Session Runtime

```text
starting -> active -> ending -> ended
              ^
              |
       potentially_stale
```

`potentially_stale` is not ended and may return to active after explicit
continuity confirmation. A Workspace or Branch switch is one atomic Runtime
Operation with validation, claim handling, Focus handling, and provenance.

- Workspace switch clears Focus by default unless a valid target Focus is
  supplied in the same operation.
- Branch switch preserves Focus only when both the Entity and context path are
  valid at the target head; otherwise it clears Focus and returns the reason.
- Session end atomically captures the final timeline boundary, generates the
  deterministic SessionDiff, releases default claims, clears Focus, and moves
  to `ended`. Failure leaves Session and Claims unchanged for retry.

## Claim Runtime

For one Task and Work Branch, the active set is exactly one of:

```text
none
one exclusive Claim
one or more shared Claims
```

Exclusive and shared Claims cannot coexist. Claim release, mode change, and
takeover are explicit atomic operations with provenance. A stale takeover and
a forced takeover are distinct; forced takeover requires rationale and records
the prior claimant, last activity, and Session state.

Each ownership/mode period is a stable immutable Claim occurrence. Active
ClaimRuntime is separate current state. Release ends the occurrence; mode
change or takeover ends the old occurrence and creates a new one.

Claiming a Task never changes its Work-State status. Atomic claim-next may
select, claim, focus, and return Context, but it does not imply TaskStart.

## Merge Runtime

```text
MergeRuntime.status = active | completed | aborted
```

`resolving` and `ready` are projections from unresolved `CONFLICT` and `REVIEW`
items. One target Workspace/Branch may have at most one active merge. Source
Branches may remain sources of other merges.

Merge start captures merge base, target head, and source head. Resolution
choices—including custom mini-ChangeSet proposals that may create Entities—are
provisional Runtime Coordination plus immutable provenance. They do not mutate
the target Work State.

Merge continue requires every `CONFLICT` and `REVIEW` resolved and both captured
heads unchanged. It then creates one atomic merged ChangeSet and a two-parent
WorkStateCommit. If either head moved, V1 rejects continue and requires restart
or a later separately designed recomputation path. Merge does not lock a
Branch; it freezes the input snapshot for optimistic validation.

Abort marks the runtime attempt aborted, creates no WorkStateCommit, and retains
the base, heads, classification, attempted resolutions, rationale, and Events.
Completed or aborted runtime state may leave the active projection while its
stable MergeAttempt, captured inputs, items, resolutions, and Events remain
immutable provenance.

## Open implementation boundary

The final command tree, protocol encoding, complete operation catalogue, exact
error-code enumeration, Claim replacement identity, source-head recomputation,
and semantic AC-waiver operation remain unfixed.
