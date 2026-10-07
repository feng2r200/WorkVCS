---
name: workvcs
description: "Use WorkVCS to recover, coordinate, version, and persist durable project work and knowledge when policy requires participation, the user asks for WorkVCS, prior state matters, or current work yields reusable cross-turn state. It supports No-Plan and Plan work; it does not replace project authority or authorize guarded actions."
---

# WorkVCS

WorkVCS is the durable provider for explicit work and knowledge state. It can
preserve semantic records, versioned work graphs, evidence, coordination,
history, handoffs, and recovery state. It does not capture model internals or
infer meaning from a transcript. The user, model, or applicable governance
Skill still decides value, scope, authorization, validation, and completion.

## Select the participation mode first

Use **required participation** when a higher-priority user, project, or
environment policy says WorkVCS must participate whenever available. Load this
Skill at the start of every task that consumes, produces, or changes
continuable project state. In this mode:

- resolve the logical project before material work and perform one bounded
  read when prior state can exist;
- persist the smallest truthful delta at narrow semantic checkpoints instead
  of waiting until closeout;
- never skip WorkVCS because a document exists, the current directory is a
  mirror or temporary location, or the task remains No-Plan; and
- reconcile any pending delta and read back the durable result before handoff
  or closeout.

A narrow semantic checkpoint occurs when the goal or scope changes, the user
confirms a decision or authority boundary, a Finding or Decision becomes
evidenced, a failed Attempt changes the route, Verification or Evidence
arrives, a Task or milestone changes state, a blocker appears or clears, or
the work pauses, hands off, or closes. Record before another step would make
reconstruction materially harder. Coalesce consecutive mechanical operations
that produce no semantic delta; do not store a transcript or chain of thought.

Use the **value-gated default** only when no higher-priority policy requires
participation. Select WorkVCS when the user asks to recall, resume, audit, or
record state; prior durable state can change the current route; or current work
produces a finding, decision, evidence item, recovery need, unresolved issue,
or reusable conclusion with a concrete consumer beyond this turn. Mere
possibility of future usefulness is not enough. If none of these conditions is
present, make no WorkVCS call.

Required participation changes selection and admission timing only. It never
forces a Plan and never authorizes migration, activation, recovery delivery,
rollback, Push, release, deployment, or another separately guarded mutation.

## Resolve ownership and recover bounded state

Resolve the logical owner in this order: explicit ProjectRef, verified
semantic Project, verified Git common directory, canonical CWD, then pending
resolution. A repository, mirror, temporary directory, coordination checkout,
or display name is context, not an override of a known higher-ranked owner.

Start with `workvcs project discover --cwd <path>`. When existing state can
change the work, use exactly one bounded path:

- `recall --profile brief` for current project context;
- `recall --profile handoff` for continuation or delegation;
- `recall --profile retrospective` for reconstruction and learning; or
- `resume --cwd` when a live or recoverable Session is relevant.

If continuation depends on one exact predecessor, use its stable identifier or
a focused Handoff; bounded Recall does not promise that every historical item
appears. Discovery, Recall, Resume, audits, and recovery status are read-only
and do not create a Plan or Session.

A trusted integration may pass bounded semantic locator evidence through
`--locator-adapter-context FILE` or already-verified generic evidence through
`--locator-context FILE`. Never manufacture either from a display name,
memory, prose, or an unverified path. Invalid or contradictory evidence fails
closed; absence may degrade to verified Git and then CWD.

When a project has an accepted stage baseline but no relevant WorkVCS state,
read [the baseline bootstrap workflow](references/workflows.md#bootstrap-live-workvcs-state-from-an-existing-stage-baseline)
before continuing. Reconstruct only what the baseline supports, preserve its
provenance and as-of boundary, read back the result, and then use WorkVCS as
the live execution-state authority. Do not invent historical events or promote
stale baseline text to current fact.

## Keep WorkVCS and project documents distinct

WorkVCS owns changing execution state: current progress, next work, pending
decisions, open execution risks, evidence, recovery, and coordination. A stage
baseline is a project-native snapshot, not a live activity log. During active
stage work, keep narrow semantic deltas in WorkVCS. After stage acceptance,
update the baseline once from verified artifacts, the work conversation, and
WorkVCS state, then record the resulting snapshot reference or evidence back in
WorkVCS.

Architecture contracts, ADRs, and other project authority follow their own
promotion rules. They may be written alongside WorkVCS when their roles are
different, but neither a document write nor a WorkVCS record silently replaces
the other.

## Use the capability that matches the work

Do not reduce WorkVCS to one generic note. It can represent small No-Plan
cognition, a versioned Goal/Plan/Task graph, acceptance and evidence, runtime
coordination, historical explanation, alternatives, recovery, and portable
state. Read [Capability routing](references/capability-routing.md) when choosing
among those modes.

For semantic object choice, currentness, and relations, read
[Semantic recording](references/semantics.md). Prefer the smallest truthful
objects and explicit provenance. `capture` can atomically create standalone
Records, Knowledge, Evidence metadata, and supported relations without a Plan.
Use Goal, Plan, Task, Acceptance Criterion, and Verification Requirement only
when their structure improves coordination, recovery, or acceptance.

For a first write, ProjectRef bootstrap, CaptureGroup, journal recovery,
Plan admission or evolution, or focused Handoff, read
[Plan and capture workflows](references/workflows.md). A missing binding is a
routing state, never evidence that admitted content is unworthy.

## Preserve control-plane safety

Before declaring WorkVCS unavailable, run `workvcs config show`. If the same
canonical registry already has a verified and active explicit-registry route,
use that exact `--registry`; do not treat configured-home versus
registry-sidecar selection as global unavailability, scan arbitrary registries,
or activate a different route. Read
[Configuration](references/configuration.md) only after an observed
configuration, locator, registry, binding, or marker problem.

Registry migration, rollback, shared-binding isolation, read-routing
activation, journal-admission activation, marker refresh, and operation-recovery
apply are separate mutation boundaries. Their preview or status forms do not
authorize apply. Use current digests, status-first recovery, and the exact
installed binary; never delete or hand-edit a stale marker, substitute a
source-tree binary for live installation, or infer authority from fixture
success. When two ProjectRefs share one exact target without proven logical
partitioning, treat reads as candidate context and fail closed on new writes
until the explicit preview/digest-locked isolation route restores a dedicated
Store. Isolation preserves the source Store and copies no historical state;
refresh both stale activation markers separately before reconstruction.

On registry v2 the journal marker grants named capabilities. Marker version 1
remains cognition-only; `plan admit --cwd` and `plan evolve --cwd` require an
explicit old-digest-locked refresh to the version-2 capability superset. The
canonical `project operation-recovery` surface serves cognition and typed Plan
operations through the same intent/event/projection/receipt protocol;
`project capture-recovery` remains a visible compatibility alias. Always read
`payload_kind`; never reinterpret a Plan payload as cognition or create a
second recovery queue.
For routed Plan output, distinguish `journal_admission_reused` from the target
`admission_status` or `evolution_status`: an already admitted intent can still
create its first target commit, while commit-before-receipt recovery reuses the
target result. Treat a terminal typed preflight as closed before receipt
construction, and require the durable Plan receipt's operation kind and full
manifest-derived result shape to match before reporting success. Once that
receipt exists, render the result through the read-only target lookup.

The journal-backed route provides no silent loss after admission. It does not
claim that a caller recognized and submitted every valuable semantic delta
while a global per-turn Hook remains deferred.

If an exact binding, routing, admission, or safety gate prevents a required
read or write, report degraded durability, retain one bounded pending semantic
packet in active context, and persist it when the accepted route is restored.
Do not silently substitute a document-only update or create a second durable
queue.

Journal admission preserves a typed intent; it does not mean the target Store
can Recall that checkpoint or Plan transition. When recording or reconciling a
continuing operation, read
[Checkpoint delivery](references/checkpoint-delivery.md) once for the selected
route to distinguish receipt work from missing ownership and complete only the
delivery already authorized. If delivery is outside scope, retain the admitted
CaptureId and report that exact boundary instead of repeatedly admitting the
same content.

The command forms in this Skill and its references are established. Do not
prepend Help calls to them. Use command help only for a genuinely different
form or after an argument rejection.

## Keep Plan, records, and authority independent

A Plan is useful for dependent stages, handoffs, confirmation contracts, or
long-running work whose state is costly to reconstruct. Complexity or impact
alone does not require one. No-Plan work may still persist Findings, Decisions,
Assumptions, Questions, Risks, Attempts, Evidence, and Knowledge. When work
evolves into a Plan, carry forward only still-relevant state and record why the
durable coordination structure became useful.

Creating or reading WorkVCS state, including a mechanical authorization
receipt, does not grant authority for the underlying external, destructive,
production, release, Push, deployment, credential, or data-changing action.
