# Plan and capture workflows

## First durable write in an unbound project

Keep discovery read-only. If it returns `project_binding_not_found`, first
confirm the logical owner using explicit ProjectRef, verified semantic Project,
Git common directory, CWD, then pending resolution. The work's artifact or
operation path is evidence and boundary context, not an automatic override of
a higher-ranked semantic owner. Do not bind an ambient mirror or temporary
working directory merely because it is the process cwd.

Value admission precedes this routing step. Preserve admitted content as one
bounded pending semantic packet when the installed CLI cannot bind the
verified semantic owner. `capture --value-reason` persists a target-neutral
`legacy_cognition_v1` intent on registry v1 or, behind both exact v2 markers,
a target-neutral `cognition_v2` intent before creating a ProjectRef or writing
a Store. Verify the installed revision and marker status rather than inferring
them from source availability. The available
`project registry-migrate --preview` command is inspection-only and is not a
capture or recovery path. The source-tree migration apply, explicit rollback,
and read-only rollback probe do not activate write routing and must not be used
on the live registry without separate authorization for the exact operation.
For a standard registry filename, rollback enumerates both home-root and
registry-sidecar activation/journal aliases; changing from configured home to
explicit `--registry` is not a bypass. The mutation candidates are currently
Unix-only. Their empty-journal proof is admissible only while v2 journal
admission remains disabled. The source route now shares one canonical-registry
quiescence lock, uses the closed alias, and repeats the exact registry identity
check around its restrictive activation callback. Enabling it still requires
installation plus a separately authorized, digest-locked
`project journal-admission-activation --apply`; disabling it requires the
exact installed marker digest. Neither action activates bootstrap, event
processing, or Store delivery.
The separate `project capture-recovery` route starts with
`--status --capture-id <id>`. Status is read-only and produces the registry
and projection digests needed by an explicit `--apply`. Apply can converge an
unbound semantic, Git, or CWD owner to exactly one ProjectRef and pristine
genesis Store binding; conflict or unresolved ownership remains pending and
does not fall back. It can then deliver target-neutral cognition under fresh
target guards. After `capture_recovery_install_indeterminate`, status is mandatory
before digest-locked forward recovery; rollback is never inferred. The
route has isolated-fixture fault validation; a live apply still needs explicit
authority for that delivery. The ordinary-read route
can consume registry v2 only after a separate, exact `project
routing-activation` marker is installed; this gate is default-off and does not
provide journal durability. The optional `--repair-manifest` only previews a
source/key/target/evidence-bound historical ownership correction; it does not
mutate the registry, rewrite a path, or activate routing.

On a verified registry-v2 journal route, admit the first valuable intent before
binding convergence and use separately authorized recovery. On the legacy v1
direct-Store route, run `project ensure` only when the first durable write is useful. For planned
work, ensure and then admit the Plan, carrying forward the useful pre-Plan
findings, decisions, unknowns, constraints, and evidence. For No-Plan work,
ensure only when a standalone semantic item is worth capturing. A failed
ensure pauses persistence, not otherwise safe work; keep a bounded pending
packet and retry after the locator/bootstrap issue is fixed. Never fall back
from a known unbound semantic Project to a bound repository merely for
convenience. The pending packet remains active-context compatibility state, not
a durability guarantee, until the candidate is installed and the accepted
journal route is separately activated.

## Standalone cognition

Use one `capture` manifest for a coherent set of new Records/Knowledge and
their relations:

```json
{
  "schema_version": 1,
  "idempotency_key": "project-specific-stable-key",
  "records": [
    {
      "local_id": "finding-1",
      "kind": "finding",
      "statement": "Observed behavior",
      "scope": {"source": "focused-probe"}
    }
  ],
  "knowledge": [
    {
      "local_id": "knowledge-1",
      "statement": "Reusable conclusion",
      "scope": {"project": "example"},
      "provenance": {"source": "finding-1"}
    }
  ],
  "evidence": [],
  "relations": [
    {
      "local_id": "validates-1",
      "type": "validates",
      "source_local_id": "finding-1",
      "target_local_id": "knowledge-1",
      "rationale": "The focused probe confirms the conclusion"
    }
  ],
  "rationale": {"reason": "preserve reusable cognition"}
}
```

Run `workvcs capture --cwd <project> --manifest <file>`. Optional expected
head/state fields add CAS guards; the CLI uses the currently verified bound
head when they are omitted. Reusing the same key with identical content returns
the prior result; a conflicting payload fails.

The journal-first form is
`workvcs capture --cwd <context> --manifest <file> --value-reason <reason>`.
It may also receive `--project-ref ID`, a strict `--locator-context FILE` with
already-verified semantic evidence, or a strict
`--locator-adapter-context FILE` from a trusted integration. On registry v2 it
may additionally receive `--capture-group FILE`; the strict group is control-
plane authority and is part of the idempotency identity. Generate its stable
ID with `workvcs id new --kind capture-group`; do not borrow another typed ID.
The first concrete
dispatch target is `codex-app-project-metadata/v1`; its bounded context may
contain verified task-Project metadata, a canonical Project-mirror path, or
both. Adapter absence yields no semantic evidence and permits Git/CWD
degradation; explicit malformed adapter input fails closed. Other tools can
add their own adapter behind the same provider interface without changing the
core resolver or the ownership order.
On registry v1 it records `legacy_cognition_v1` and reports migration required;
CaptureGroup input is rejected. On registry v2 it requires both exact
activation markers, removes manifest transport fields, rejects caller target
head/state guards, and records only immutable `cognition_v2`. Do not substitute
a repository-local binary for the installation and live-activation gates.

Primary delivery is a separate operator action, never an automatic continuation
of `capture`: first run
`workvcs project capture-recovery --status --capture-id <id>`, then use the
fresh registry/projection digests with `--apply` only inside the explicitly
authorized control plane. Apply records target guards before the Store commit;
if status remains `pending_primary` after an uncertain result, replay the same
capture so target idempotency can recover a committed-but-unreceipted result.
Receipt reuse across a registry refresh additionally requires the same
ProjectRef and exact Store/Workspace/Branch target.
`legacy_manifest_upgrade_required` requires a separate manifest-upgrade
decision. At `pending_references`, use fresh status digests and repeat the
explicit apply: it validates each member ProjectRef and installs only missing
immutable-reference events without opening a secondary Store. Recall an
association read-only with
`workvcs project capture-group-recall --project-ref-id <id> [--registry <path>]`.
Recovery has isolated-fixture validation but remains a live write operation;
require current status digests and explicit authority for the exact apply.

## No-Plan to Plan

Admit a Plan only after governance concludes that durable planning now adds
value. The first `plan admit` manifest should include:

- an explicit upgrade reason in rationale;
- the Goal and selected strategy;
- only useful Tasks and acceptance/verification requirements;
- earlier Findings, Decisions, Questions, constraints, and Evidence that the
  Plan still depends on.

Use `plan evolve mode=in_place` for additive or non-contract-breaking changes.
Use `mode=supersede` when the confirmed Plan contract is being replaced and its
ancestry must remain visible. Do not maintain parallel Plan versions merely as
a testing ritual.

## Recall and retrospective

Every Recall profile first reserves one representative from each non-empty
active Goal, active Plan, live Session, active Claim, and non-terminal Task
category before any category can consume the remaining budget. Live Sessions
include both `active` and recoverable `potentially_stale` states; a large Task
queue therefore cannot hide all current ownership detail. `brief` is for immediate
active context. `handoff` then adds the newest semantic context needed by
another Agent. `retrospective` samples the newest Records, Knowledge, semantic
relations, and Store-wide Evidence metadata across categories before remaining
semantic history and terminal Goal/Plan/Task summaries. A bounded projection
therefore cannot be consumed by one large category or old work merely because
it was recorded first.

Brief Recall includes only current Record states. Handoff Recall additionally
keeps terminal Attempts because they prevent duplicate work. Retrospective
Recall preserves terminal Findings and other history, but prioritizes current
Records and Knowledge so historical statements do not masquerade as live
project truth.

Bounded Recall does not guarantee that a particular terminal Task appears. If
continuation depends on an exact predecessor outcome, pass the stable Task or
Evidence id in the delegation contract, or create a focused Handoff and query
that object directly.

Use each item's `temporal_scope`, version/state digest,
`snapshot_commit_id`, Record scope, and Knowledge scope/provenance to
distinguish current Work State, live Runtime Coordination, and historical Store
inventory. `snapshot_commit_id` is the evaluation point, not the object's
creation time; Recall uses current version identifiers for newest-first semantic
ordering. For closeout, a branch source has
`runtime_temporal_scope=live_read_time`; an explicit commit has
`runtime_temporal_scope=historical_commit`. If evidence body matters, inspect
the item and extract its persisted content separately; do not place arbitrary
large bodies into every recall response.

Recall answers “what context should I recover now?”; currentness audit answers
“which explicit open obligations or current claims merit deliberate review?”
Use `record currentness-audit` only when that review has value. It is bounded
and read-only, and it neither replaces retrospective Recall nor turns Plan
closeout into a Record-lifecycle gate.

When a mutating command returns `mutation_postcondition_failed`, the mutation
already completed but a result-dependent `--expected-*` assertion did not.
Inspect `operation_result`, recover the current state, and decide whether any
new action is still needed. Do not blindly replay the mutation.
