---
name: workvcs
description: "Use WorkVCS when prior project state can affect current work, or when work develops durable value that should survive a turn, handoff, or restart: meaningful findings, decisions, evidence, recovery context, coordination, or a Plan. Do not use it merely because the command is installed."
---

# WorkVCS

Use WorkVCS as an optional durable work-memory provider. It records mechanics;
the model or an applicable governance Skill still decides policy, scope,
authorization, validation strength, and completion.

## Decide from durable value

Do not probe WorkVCS at the start of every task. Invoke it only when at least
one condition is present:

- the user asks to recall, resume, audit, or record WorkVCS state;
- prior durable state can change the current route, ownership, authority, or
  completion judgment;
- current work produces a meaningful finding, decision, failed route,
  evidence item, unresolved question, or reusable conclusion that should
  survive a turn, handoff, or restart; or
- durable coordination or recovery would materially help the work.

When none applies, make no WorkVCS call and create no WorkVCS state. Reassess
internally when the route, scope, or ownership changes and before a handoff,
long pause, or meaningful closeout. This reassessment is not a required CLI
step. When value emerges during work, invoke WorkVCS just in time and prefer
one coherent capture over incremental narration.

Examples:

- a routine edit with no useful prior state or durable outcome: no call;
- a small fix that uncovers a reusable root cause: capture that result without
  admitting a Plan merely because it was recorded;
- an explicit continuation request: use one bounded recall or resume path; and
- multi-stage work: admit a Plan only if its coordination or recovery value
  justifies the durable structure.

## Read only what can affect the task

After the value decision, resolve the logical project that owns the work and
run `workvcs project discover --cwd <path>`. Prefer an explicit user target or
the primary artifact/operation directory over an ambient ChatGPT mirror,
temporary directory, or coordination checkout. If location or binding
integrity is in doubt, run `workvcs project list --require-valid`.

Choose one bounded read path:

- `recall --profile brief` for active context;
- `recall --profile handoff` for a continuation or another Agent;
- `recall --profile retrospective` for reconstruction and learning; or
- `resume --cwd` when an active Session is relevant.

Read-only discovery, recall, audit, and recovery do not create a Plan or
Session. Do not scan the whole Store when a bounded projection is enough.

Use `command -v workvcs` only when availability is unknown. Use exact command
help before relying on flags.

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

- Use [Plan and capture workflows](references/workflows.md) for unbound first
  writes, atomic capture, Plan admission/evolution, Recall, Handoffs, and
  mutation recovery.
- Use [Semantic recording](references/semantics.md) for object choice,
  relations, lifecycle closure, and bounded currentness review.
- Use [Configuration](references/configuration.md) only for setup or locator
  problems.

Prefer raw content when WorkVCS should preserve an evidence body; digest-only
Evidence may not be locally extractable. For shared work, default to one writer
per semantic slice and pass stable IDs or a focused Handoff instead of copying
an unbounded conversation.

## Do not confuse records with authority

Recording WorkVCS state, including a mechanical authorization receipt, needs no
extra permission by itself. It does not authorize the underlying external,
destructive, production, release, push, deployment, or credential action.
Apply the governing authority boundary to that real action.
