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
owns the work and run `workvcs project discover --cwd <path>`. Prefer an
explicit user target or the primary artifact/operation directory over an
ambient ChatGPT mirror, temporary directory, or coordination checkout. If
location or binding integrity is actually in doubt, run
`workvcs project list --require-valid`.

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

If discovery returns `project_binding_not_found`, continue otherwise safe
work. Run `workvcs project ensure --cwd <logical-project>` only immediately
before admitting a useful Plan or capturing a valuable standalone item.

Ensure creates only the Store/Workspace/Branch binding, not a Plan or semantic
record. If it fails, keep one small pending semantic packet in the active work
context, report the exact problem, and persist it after recovery. Do not create
a second durable queue or silently discard the packet.

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
