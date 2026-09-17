---
name: workvcs
description: Use whenever the workvcs command is available for a project, unless the user explicitly opts out. Provides low-friction durable cognition, project recall, Goal/Plan/Task state, evidence, retrospective support, and multi-agent context exchange without making Plan mandatory.
---

# WorkVCS

Use WorkVCS as an optional durable work-memory provider. It records mechanics;
the model or an applicable governance Skill still decides policy, scope,
authorization, validation strength, and completion.

## Start cheaply

1. Confirm availability with `command -v workvcs`.
2. Resolve configuration with `workvcs config show`.
3. Resolve the logical project that owns the work, then run
   `workvcs project discover --cwd <path>`. Prefer an explicit user target or
   the primary artifact/operation directory over an ambient ChatGPT mirror,
   temporary directory, or coordination checkout. If location or binding
   integrity is in doubt, run `workvcs project list --require-valid`.
4. Read only what the current task needs:
   - `recall --profile brief` for active context;
   - `recall --profile handoff` for a continuation or another Agent;
   - `recall --profile retrospective` for reconstruction and learning;
   - `resume --cwd` only when an active Session is actually relevant.

Read-only discovery, recall, audit, and recovery do not create a Plan or
Session. Do not scan the whole Store when a bounded projection is enough.

If discovery returns `project_binding_not_found`, do not conclude that the
work cannot be recorded and do not interrupt unrelated execution. Use
`workvcs project ensure --cwd <logical-project>` immediately before the first
durable write that has value:

- when governance selects a Plan, ensure first and then admit it;
- for No-Plan work, wait until a useful standalone Record, Evidence item, or
  Knowledge statement actually exists, then ensure and capture it.

Ensure is idempotent and creates only the Store/Workspace/Branch binding, not a
Plan or semantic record. If ensure cannot complete, retain the small pending
semantic packet in the active work context, report the exact locator or
bootstrap problem, and persist it after the provider recovers. Do not create a
second durable queue or silently discard the packet.

## Keep Plan and persistence independent

A Plan is a governance choice, not the admission ticket for WorkVCS. Small or
single-step work may stay No-Plan and still persist a valuable Finding,
Decision, Assumption, Question, Attempt, Evidence item, or Knowledge statement.
Use `workvcs capture` to create related standalone cognition atomically.

When work genuinely evolves from No-Plan to Plan, the first admission must say
why durable planning became useful and carry forward the still-relevant
findings, decisions, unknowns, constraints, and evidence. See
[Plan and capture workflows](references/workflows.md).

## Record meaning, not narration

Persist information when it improves continuation, review, audit, or future
work. Prefer semantic records and explicit relations over a chronological
transcript. Capture the original problem, hypotheses, discoveries, choices and
tradeoffs, route changes, failed attempts, unresolved questions, evidence, and
reusable conclusions when they matter.

Do not invent relations merely to fill a graph. Use `supports`, `contradicts`,
`derived_from`, `validates`, `invalidates`, or `supersedes` only when the stated
causal or epistemic claim is justified. See
[Semantic recording](references/semantics.md).

Close a running Attempt when its result becomes known. If a Finding is later
corrected or disproved, do not leave two contradictory Findings current: use
the guarded Finding correction command that preserves the prior statement and
creates its justified evolution relation.

When a Question becomes answered, deliberately deferred, or irrelevant, close
it with `record question-status`. When a Risk is mitigated, disproved, or no
longer tracked, close it with `record risk-status`. Do not leave resolved
unknowns or exposures current merely because retrospective history should be
preserved.

When resumed work, closeout, periodic review, or contradictory Recall makes a
currentness review useful, run one bounded `record currentness-audit`. Start
with its default open obligations; add `--include-current-claims` only when
Findings, Decisions, and validated Assumptions also need review. Treat every
candidate as a prompt for explicit judgment, not proof of staleness. Retaining
a still-current Record is valid, and the audit never creates Plan gaps. See
[Semantic recording](references/semantics.md).

## Evidence must remain inspectable

Prefer raw content input when WorkVCS should preserve the evidence body. Check
metadata first with `evidence show`; use `evidence extract` to recover a
persisted body and verify its digest. Digest-only Evidence intentionally records
an external content identity and may not be locally extractable.

## Coordinate multiple Agents through shared truth

Any Agent may use bounded read-only recall. Default to one writer for the same
semantic slice—normally the parent Agent or an explicitly designated recorder.
Use Claims for contested executable Tasks, not for every read or every record.
Handoffs should reference stable IDs and current head/digest instead of copying
an unbounded conversation.

If delegated work depends on proving a predecessor's exact terminal outcome,
provide its stable Task/Evidence identity or a focused Handoff. Generic bounded
Recall is optimized for current work and does not promise that an arbitrary
terminal Task will fit in the projection.

## Do not confuse records with authority

Recording WorkVCS state, including a mechanical authorization receipt, needs no
extra permission by itself. It does not authorize the underlying external,
destructive, production, release, push, deployment, or credential action.
Apply the governing authority boundary to that real action.

Load [Configuration](references/configuration.md) only for setup or locator
problems. Use exact command help before relying on flags.
