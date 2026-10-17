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

### Finding, Assumption, Attempt, Decision, Question, and Risk

```text
Finding:
  active -> superseded | invalidated

Assumption:
  unverified -> validated
  unverified -> invalidated
  validated  -> invalidated

Attempt:
  running -> succeeded | failed | inconclusive

Decision:
  active -> superseded | withdrawn

Question:
  active -> answered | deferred | withdrawn

Risk:
  active -> mitigated | invalidated | withdrawn
```

Finding supersession atomically moves the prior Finding to `superseded` and
creates `replacement -> prior` `supersedes`. Finding invalidation atomically
moves the target Finding to `invalidated` and creates
`cause -> target` `invalidates`. Both source Findings must already be active,
and terminal Findings cannot transition again.

An invalidated Assumption is not ordinarily revalidated; changed conditions
produce a new or superseding Assumption. A terminal Attempt is never reopened;
a later try is a new Attempt. An old Decision is never reactivated. Re-adopting
an earlier choice creates a new Decision that supersedes the currently active
Decision. Explicit Decision supersession does not require identical
`scope+subject`, and matching `scope+subject` does not itself imply
supersession.

An answered, deferred, or withdrawn Question is no longer a current unknown;
reopening the inquiry creates a new Question. A mitigated, invalidated, or
withdrawn Risk is no longer a current exposure; changed conditions create a
new Risk. The terminal transition rationale records why the currentness
changed without rewriting the original statement.

Major transitions carry rationale text, a causal Entity reference, or both as
defined by their semantic contract. Coordination `force` cannot bypass a
mandatory AC gate; any waiver is a distinct semantic concept.

### Record currentness audit projection

Currentness review is a query, not another semantic state machine. The default
projection selects only explicit open lifecycle states: unverified Assumptions,
running Attempts, active Questions, and active Risks. An explicit wider mode
also selects validated Assumptions and active Decisions and Findings. Kind,
exact canonical scope, and statement-substring filters narrow those classes.

The query returns newest-current-version-first candidates under a default
50-item and hard 200-item budget, with total and omitted counts. It opens the
Store read-only and never infers staleness, performs a transition, or creates a
Plan gap. Branch sources represent the resolved current head. Commit sources
represent historical state and are inspection-only; a historical result is not
a safe mutation base.

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

## Continuation Rotation Runtime

Continuation rotation is scoped to one Workspace, Work Branch, Task, and
exclusive Claim. The scope projection holds one listener and monotonically
increasing listener revision, one current carrier, one current Claim, and a
strictly increasing owner epoch. It is not a Project-global owner or a
Branch-wide write lock.

```text
pending_creation -> creation_dispatched -> creation_unknown
                               |              |
                               |              +-> creation_succeeded
                               +-----------------> creation_succeeded
                                                    |
                                                    v
                                            candidate_attached
                                                    |
                                                    v
                                            candidate_accepted
                                                    |
                                                    v
                                                committed
```

An exact Project first requires a current, enabled, revisioned WorkVCS
continuation policy backed by a trusted product-control-plane envelope that
binds separate user-authorization and Project-membership receipts. Only the
bound listener may admit the attempt. Admission is idempotent and one scope
has at most one active rotation. The occurrence binds its listener
revision, source epoch and Claim, Handoff digest, creation operation/request,
typed currentness basis, current Project-policy receipt, bounded continuation
packet, initial-message digest, and the embedded deterministic evaluator
receipt with complete gate summary and bounded evidence references. The source
transcript is explicitly absent.
Before the product call, dispatch is durably recorded, the Project policy is
reread, the Branch projection is refreshed, and admitted scope currentness is
recomputed so revocation or relevant drift fences the effect. The resulting receipt then records
`unknown` or `created`; unknown-to-created reconciliation binds the immutable
unknown receipt and client identity rather than retrying creation.

Candidate attachment records a `work_candidate` Session and destination-binding
evidence but does not move the Claim. The
binding must prove a non-forking `create_thread` result correlated to the
admitted operation/request, a product receipt, exact Project provenance, and a
stable normal Work thread that is user-visible, auditable, and interactable. A
pending client ID, fork, hidden/background task, SubAgent, side chat, or
WorkVCS-only identity cannot advance the state. The target Session must then
author an acceptance bound to the current listener revision, Handoff,
currentness, packet digest, and initial-message digest.

Commit is one `BEGIN IMMEDIATE` transaction. It validates the active rotation
and accepted candidate, compare-and-swaps the exact scope inputs, replaces the exclusive
Claim occurrence for the candidate, increments the scope epoch, writes terminal
provenance, and removes the active attempt projection. Abort leaves the source
Claim and epoch unchanged. A created target thread is reserved from the first
successful receipt even when its attempt later aborts; it cannot become a
listener or ordinary Work carrier/source. Only its exact committed candidate
Session and resulting Claim may source later work in the exact scope.

Normal listener replacement is explicit, revision-fenced, and can recover an
active attempt only when the non-listener currentness basis has not drifted. If
the target had accepted, its prior acceptance is archived and it must accept
again under the replacement listener revision. If the old listener is inactive
and normal recovery is unsafe, abort-only recovery preserves the original
decision/currentness and all receipts, records observed currentness under a
monotonic recovery revision, and permits only original-operation
reconciliation plus state-specific abort. It can never dispatch, attach,
accept, or commit, and a replacement recovery listener cannot leave that mode.
Preview supplies authority-bound observed currentness; apply rechecks it in the
same transaction. Full recovery receipts form an append-only Event digest
chain, while normal snapshots expose a fixed-size count and chain head.
Listener loss does not affect the carrier's Claim or ordinary work. A pre-commit
candidate has no transferred Claim and cannot initiate a new rotation; after
commit it may source a later rotation only through its exact committed Claim
provenance. Ordinary Claim and Session departure paths cannot bypass an active
attempt.

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
