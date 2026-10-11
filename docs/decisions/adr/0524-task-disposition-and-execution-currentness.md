# ADR-0524: Task disposition and execution currentness

Status: Accepted — implemented in local source; stable adoption evidence pending
Date: 2026-10-10
Accepted: 2026-10-11

## Context

Replacing or retiring a Plan can leave its non-terminal Tasks reachable only
through the old structural path. Treating every such Task as automatically
retired loses valid work; treating every Task as still current makes historical
work appear executable. A Plan lifecycle state therefore cannot by itself
answer whether a Task is current or runnable.

The user-confirmed requirement is narrower: before a fresh supersede or a
repair of a retired Plan, the affected Task inventory must be assessed
explicitly. An assessment may keep an old Task unchanged when that is the
truthful result. This requirement does not authorize execution, budgets,
claims, handoffs, external actions, or repair of any particular project's
existing state.

## Decision

The local source implementation introduces schema-version-2 Plan evolution for two
manifest modes: `supersede` and `task_disposition`. A fresh schema-2
supersede replaces an active Plan. `task_disposition` repairs the bounded
Task inventory of a `superseded` or `abandoned` Plan. Both require branch and
target Plan compare-and-swap guards, and both enumerate every in-scope
non-terminal Task exactly once. A guarded continuation at the highest active
nested-Plan frontier covers that subtree once, rather than requiring a second
per-Task disposition from the outer replacement.

Each disposition is one of `supersede`, `cancel`, `continue`, or
`retain_original`, and carries the Task identity, expected version, expected
state digest, return condition, and structured rationale. `supersede` creates
the declared replacement Task and its machine `supersedes` relation;
`cancel` terminalizes the old Task; `continue` preserves the old Task and
uses the new replacement Plan in a supersede or a guarded existing current
Goal/Plan carrier in a retired-Plan repair; and
`retain_original` preserves its identity, status, version, and state digest.
The latter writes only the immutable assessment result. A valid already-existing
current Goal/Plan reference may keep it current; historical or deferred
retention is also valid. Neither result fabricates an Entity update or adds or
replaces a carrier merely to make the Task look current.

For the narrow all-retain repair case, the implementation records a state-preserving
normal commit with no ChangeOperation. This is permitted only for a non-empty
schema-2 `task_disposition` manifest whose dispositions are all
`retain_original`, whose task/relation result sets are empty, and whose result
has no replacement Plan or Plan relations. The immutable assessment is carried
by the ChangeSet operation payload and is replayed strictly. This is not a
general permission to create empty commits.

The implementation evaluates pre- and post-operation currentness with one
fixed-point resolver. Active Goals, active standalone Plans, and non-terminal
standalone Tasks are roots. Primary containment and structural references from
reachable active Goals or Plans carry scope; inactive Goals and Plans stop
that traversal. A terminal Task may still be a structural container: its
non-terminal descendants remain scope-reachable through a current Goal or
Plan carrier, while that terminal Task itself is neither current nor runnable.
Rootless reference cycles do not activate themselves. Default brief and
handoff views exclude non-current Tasks; retrospective views retain labelled
history and the explanatory path.

Task-origin structural references remain unsupported. A Goal or Plan may
reference a Task or Plan; Task-to-Task primary containment is the existing
decomposition mechanism. This keeps execution currentness separate from Task
lifecycle status and prevents a Task reference cycle from becoming an
unbounded carrier.

The operation is one transaction over the branch-head CAS, affected Entity and
Relation versions, current structural references, scheduling dependencies, and
ClaimRuntime checks. It rejects any newly created currentness or dependency
regression, a terminalization that breaks a surviving current reference or
dependent, and the loss of an active claim. It may record an already-existing
blocker when a pure `retain_original` leaves a historical Task unchanged; that
truthful report must not force a status change or synthetic carrier.

Nested Plans are assessed at the highest active frontier. A current nested
Plan must be explicitly continued with its own guarded version/digest; its
subtree is then covered once rather than being double-disposed by an ancestor.

The bounded preview command is:

```text
workvcs plan task-disposition-audit \
  [STORE --branch BRANCH | --cwd PATH] --plan PLAN_ID
```

It is read-only. Its output pins the head, Plan version/digest and status,
bounded descendants, currentness carriers, reverse structural references,
external dependents, ordering hints, nested Plans, and active/stale-scope
claims. It writes neither Store state nor a journal event.

## Compatibility and recovery

New schema-2 authoring uses the independently typed journal payload
`plan_evolve_v2`, schema-2 `plan.evolve` changesets/events, and a receipt whose
`operation_payload_kind` matches the admitted intent. The schema-2 payload
uses its own digest domain. Fresh schema-1 supersede authoring is rejected
before journal admission.

Pure schema-2 manifest defects fail before journal admission. Inventory
coverage and target-dependent version, current-reference, dependency, claim,
or branch-CAS failures need a target read. On a routed path they can therefore
leave immutable admitted/recovery evidence while producing no target Store
write; the direct Store path has no journal entry and also leaves the Store
unchanged.

Existing schema-1 `plan_evolve_v1` payloads remain immutable. Their exact
results, receipts, and commit-before-receipt recovery retain the legacy parser
and execution semantics; schema-1 in-place evolution also remains supported.
Missing schema-2 disposition fields are never defaulted into a legacy result.

This authoring incompatibility requires the local source package version to be
1.0.0. It establishes the version boundary, while stable installation and
adoption remain separately evidenced.

## Consequences and boundaries

The implementation makes a Plan replacement auditable without conflating historical
retention, lifecycle status, and executable scope. It deliberately does not
choose a business disposition, mutate a different project's Tasks, or turn a
continued Task into a claim, handoff, budget, or execution authorization.

The user confirmed the assessment/retention requirement and accepted the typed
manifest, resolver, journal, and receipt mechanics described here on
2026-10-11. That acceptance does not by itself establish installation,
stable adoption, public release, or authorization for external execution.
