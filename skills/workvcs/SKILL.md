---
name: workvcs
description: "Use WorkVCS only for an explicit recall/resume/audit/record request, when already-known durable project state can affect current work, or after current work has actually produced a durable finding, decision, evidence, recovery need, or coordination need. Do not invoke it prospectively at routine task start merely because work might become reusable, and do not make a Plan mandatory."
---

# WorkVCS

Use WorkVCS as an optional durable work-memory provider. It records mechanics;
the model or an applicable governance Skill still decides policy, scope,
authorization, validation strength, and completion.

## Decide from durable value

Activation is evidence-gated. Do not select or probe WorkVCS at routine task
start merely because useful state might appear later. Invoke it only when at
least one condition is already present:

- the user asks to recall, resume, audit, or record WorkVCS state;
- prior durable state can change the current route, ownership, authority, or
  completion judgment;
- current work produces a meaningful finding, decision, failed route,
  evidence item, unresolved question, or reusable conclusion that should
  survive a turn, handoff, or restart; or
- durable coordination or recovery would materially help the work.

Prospective applicability is not a condition. If this Skill was loaded before
any condition is evidenced, make no WorkVCS call, load no reference, and return
to the task. Reassess internally when the route, scope, or ownership changes
and before a handoff, long pause, or meaningful closeout. This reassessment is
not a required CLI step.

For value produced by current work, require an evidenced result and a concrete
consumer beyond the current turn. Examples include a shared invariant used by
multiple named consumers, a confirmed decision that constrains later work, or
a root cause/evidence item whose absence would make a likely recurrence or
continuation repeat the investigation. Mere possibility of future usefulness
does not qualify. When this test is met, finish validation and perform one
coherent capture before the final response; a response-only summary is not the
durable record.

Examples:

- a routine edit with no useful prior state or durable outcome: no call;
- a small fix that uncovers a reusable root cause: capture that result without
  admitting a Plan merely because it was recorded;
- an explicit continuation request: use one bounded recall or resume path; and
- multi-stage work: admit a Plan only if its coordination or recovery value
  justifies the durable structure.

## Read prior state only when it can matter

When prior durable state can affect the task, resolve the logical project that
owns the work and run `workvcs project discover --cwd <path>`. Determine
ownership in this order: an explicit ProjectRef selected for the operation,
verified semantic Project context, verified Git common directory, canonical
CWD, then pending resolution. The directory remains useful context, but an
ambient repository, mirror, temporary directory, or coordination checkout
must not replace a known higher-ranked semantic owner. If location or binding
integrity is actually in doubt, run `workvcs project list --require-valid`.

After the exact ProjectRef-v2 source candidate is installed and read routing
is separately activated, a trusted integration may pass its bounded adapter
handoff with `--locator-adapter-context FILE`. The current concrete adapter
accepts verified Codex task-Project metadata and/or a canonical ChatGPT
Project-mirror path, but it runs behind the same tool-neutral provider
interface used by future integrations. `--locator-context FILE` remains the
generic path for semantic locator evidence already verified outside WorkVCS.
Do not manufacture either file from a display name, model memory, arbitrary
prose, or an unverified path. If the provider is unavailable, omit its evidence
and continue with verified Git then CWD; if supplied adapter material is
malformed or contradictory, preserve the diagnostic and fail closed rather
than silently discarding it.

Describe the durability boundary exactly: the journal-backed candidate
provides **no silent loss after admission**. It does not claim complete
cognition capture while the global per-turn Hook remains deferred.

Then choose one bounded read path:

- `recall --profile brief` for active context;
- `recall --profile handoff` for a continuation or another Agent;
- `recall --profile retrospective` for reconstruction and learning; or
- `resume --cwd` when an active Session is relevant.

Read-only discovery, recall, audit, and recovery do not create a Plan or
Session. Do not scan the whole Store when a bounded projection is enough.

When durable value emerges from otherwise standalone work, finish and validate
that work before routing the capture. Discover the owning project immediately
before the write. Do not Recall merely because a new standalone capture is
valuable; Recall only when existing state could change its statement, scope,
duplicate/conflict judgment, relation target, or provenance.

The command forms shown in this Skill and its linked workflows are established.
Use them directly. A preflight Help call for the shown `project discover`,
`resume`, or `capture` forms is a routing error. Use
`workvcs <command> --help` only for a different form whose syntax is genuinely
unknown or after a command rejects its arguments. Do not probe availability or
prepend Help to an established form unless an actual failure requires it.

## Write just in time

Run the value decision before binding resolution. A missing binding is a
routing state, never evidence that admitted content is unworthy. ADR-0513
accepts journal-first routing and semantic-Project-before-Git/CWD ownership.
The core accepts bounded, tool-neutral adapter locator input, and the
source-tree CLI has default-off, digest-bound candidates for ordinary v2 reads
and journal admission. The CLI integration layer now includes the first
concrete provider adapter and exposes its strict dispatch envelope as
`--locator-adapter-context FILE`; the core schema and resolver remain
provider-neutral. Its actual `capture` caller can persist an explicitly
value-qualified target-neutral v1 intent, or a registry-coupled v2 intent after
both exact markers are active. It does not bootstrap a ProjectRef or write a
target Store. The candidate has not been installed or enabled against the live
control plane, so this Skill must not treat it as current live durability.

Use `workvcs project registry-migrate --preview` with optional `--registry`,
`--repair-manifest`, and `--format text|json` only when the task explicitly
requires migration inspection or registry-v2 readiness evidence. A repair
manifest is appropriate only after current evidence proves that an exact v1
path locator is historical context for a namespaced semantic Project; it must
pin the source digest, binding key, target digest, and evidence digest. The
preview is strictly read-only and does not repair a missing binding or change
the live registry. The source tree contains digest-locked migration apply and
rollback candidates, a read-only rollback state probe, versioned ordinary
reads, a separately digest-bound `project routing-activation` candidate, and a
separate `project journal-admission-activation` candidate with exact apply and
disable locks.
They are not installed or live-authorized and have been validated only on
isolated fixtures. Never infer authority to migrate, roll back, or activate
from a successful preview or probe. Do not run any mutation path on a live
registry unless the user separately authorizes that exact operation.
`--rollback-check` never performs rollback; `--rollback` is explicit, preserves
an exact v2 snapshot, and requires both digest locks plus proof of zero v2 use.
For a standard registry name, that proof checks both WorkVCS-home and
registry-sidecar activation/journal aliases regardless of which input selected
the registry. Apply and rollback currently require Unix atomic-replace
semantics. Treat the empty-journal proof as valid only while v2 admission is
disabled. The source route derives one canonical-registry quiescence lock for
every journal alias, uses the closed registry-coupled alias constructor,
supplies the observed v2 revision/digest to the mandatory core check, and
repeats the identity check after the restrictive activation callback. The
read-routing marker still activates reads only. The distinct journal-admission
marker permits intent admission only; it leaves ProjectRef bootstrap,
journal-event processing, and Store delivery disabled. Neither marker may be
installed or removed on a live control plane without separate exact authority.

The source tree also has an explicit `project capture-recovery` candidate.
`--status --capture-id <id>` is read-only: it validates immutable events,
derives the projection, compares stored projection state, and re-resolves the
owner. `--apply` additionally requires the exact observed registry and
projection digests. It may converge one unbound semantic/Git/CWD owner to one
ProjectRef and pristine Store binding, then record exact Branch guards and
deliver one canonical primary cognition result through the existing
idempotent capture engine. A target commit without a receipt is recovered by
the same target key; a stale legacy manifest stays unchanged and requires an
explicit upgrade. It never falls back after conflict. For a CaptureGroup it
derives one primary from exact locator evidence, preserves the canonical
receipt, and appends only missing immutable secondary references; it never
opens a secondary Store. The read-only
`project capture-group-recall --project-ref-id <id>` candidate scans immutable
journal authority and returns references pinned to one canonical Record
version. A registry refresh reuses a receipt only after the same ProjectRef and
exact Store/Workspace/Branch are revalidated; a delivered CaptureGroup cannot
be retargeted and requires a new capture. An indeterminate install or target
step always returns
to `--status` and forward recovery; it never authorizes rollback. This
candidate is uninstalled and fixture-only. Do not use it on the live registry
or treat the source candidate as live target-delivery durability.

If discovery returns `project_binding_not_found`, continue otherwise safe
work. Run `workvcs project ensure --cwd <logical-project>` only immediately
before admitting a useful Plan or capturing a valuable standalone item.

Ensure creates only the Store/Workspace/Branch binding, not a Plan or semantic
record. If it fails, keep one small pending semantic packet in the active work
context, report the exact problem, and persist it after recovery. Do not create
a second durable queue or silently discard the packet. When verified semantic
Project context exists, do not replace it with a repository or ambient mirror
merely because current v1 has a binding there. This active-context packet is
not durable WorkVCS state; disclose that limitation until the source candidate
is installed and its journal route is separately activated. Do not run a
repository-local candidate against the live registry as a substitute for
installation and activation gates.

## Keep Plan and persistence independent

A Plan is a governance choice, not the admission ticket for WorkVCS. Select
one when durable coordination or recovery adds value, such as dependent stages
or owners, cross-turn continuation, a confirmation contract that must survive
handoff, or long-running work whose state is costly to reconstruct. Task size,
complexity, impact, or labels such as architecture, schema, and migration do
not decide Plan admission by themselves. A small task can need a Plan, while a
large or high-impact action can remain No-Plan when its durable route is not
useful.

High-impact work still requires exact user authority, the relevant accepted
ADR or domain contract, confirmation gates, and proportional evidence. Those
requirements apply whether or not a Plan exists, and a Plan never grants
authority for the underlying action.

No-Plan work may still persist a valuable Finding, Decision, Assumption,
Question, Attempt, Evidence item, or Knowledge statement. Use `workvcs capture`
to create related standalone cognition atomically.

When work genuinely evolves from No-Plan to Plan, the first admission must say
why durable planning became useful and carry forward the still-relevant
findings, decisions, unknowns, constraints, and evidence. See
[Plan and capture workflows](references/workflows.md).

## Route detail only when needed

Persist information when it improves continuation, review, audit, or future
work. Prefer the smallest truthful semantic records and justified relations
over a chronological transcript. Capture the original problem, discoveries,
choices and tradeoffs, route changes, failed attempts, unresolved questions,
evidence, and reusable conclusions only when they matter.

References are not a startup checklist. Load only the one that answers a
current need; load another only when a later decision or failure requires it.

- Use [Plan and capture workflows](references/workflows.md) when constructing a
  first durable write, atomic capture, Plan admission/evolution, focused
  Handoff, or mutation recovery.
- Use [Semantic recording](references/semantics.md) when object choice,
  relations, lifecycle closure, or bounded currentness review is unresolved.
- Use [Configuration](references/configuration.md) only after an observed setup,
  locator, registry, or binding-integrity problem. Do not load it before
  ordinary discovery, Recall, Resume, or capture.

Prefer raw content when WorkVCS should preserve an evidence body; digest-only
Evidence may not be locally extractable. For shared work, default to one writer
per semantic slice and pass stable IDs or a focused Handoff instead of copying
an unbounded conversation.

## Do not confuse records with authority

Recording WorkVCS state, including a mechanical authorization receipt, needs no
extra permission by itself. It does not authorize the underlying external,
destructive, production, release, push, deployment, or credential action.
Apply the governing authority boundary to that real action.
