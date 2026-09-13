# ADR-0510: Bounded Record Currentness Audit

Status: Accepted
Date: 2026-09-13

## Context

Completed Plans and zero closeout gaps do not imply that independently useful
semantic Records are still accurate. Maintained-Store review found active
Risks and Findings whose underlying problems had already been resolved by later
work. Their history was valuable, but leaving their lifecycle state unchanged
made bounded Recall present some of them as current project truth.

`record list` can inspect one kind or status at a time, but a caller otherwise
has to issue several queries, join the results, recover full statements and
scope, and remember which lifecycle transitions are valid. Extending Plan
closeout would incorrectly couple independent cognition to Plan completion,
while inferring staleness from wording, recency, or terminal Tasks would turn a
mechanical tool into an unreliable policy judge.

## Decision

1. Add `workvcs record currentness-audit` as a bounded, read-only projection.
   It resolves either a verified project binding through `--cwd`, or an
   explicit Store plus exactly one Branch or Commit source.
2. The default projection includes explicit open obligations only:
   unverified Assumptions, running Attempts, active Questions, and active
   Risks. `--include-current-claims` additionally includes validated
   Assumptions and active Decisions and Findings.
3. Optional kind, exact canonical scope, and statement-substring filters narrow
   the selected lifecycle classes. The default budget is 50 Records, the hard
   maximum is 200, candidates are newest-current-version first, and output
   reports total, returned, and omitted counts.
4. Every returned item preserves its full statement, canonical scope, stable
   Record and current-version IDs, state digest, kind, status, audit class, and
   valid review actions. A Branch source is labelled as a current-head view; a
   historical Commit is inspection-only and never emits a mutation-eligible
   claim.
5. The projection uses OS-level read-only plus database `query_only` access. It
   does not transition Records, create Events or Work-State commits, infer that
   a Record is stale, or decide whether the caller should retain or change it.
6. Record currentness remains independent of Goal, Plan, Task, Verification,
   and closeout state. This command never adds Plan gaps or becomes a universal
   Plan-completion gate. Governance decides when the review has enough value to
   run and whether current evidence justifies an explicit lifecycle operation.

## Consequences

- Agents can review the most actionable semantic debt with one bounded query
  instead of reconstructing it from multiple lists.
- “Retain” is a valid review result; the command creates no mechanical churn
  merely because a Record was inspected.
- Current claims are opt-in because they are normally more numerous and less
  immediately actionable than open obligations.
- Historical cognition stays available and unchanged after an explicit
  correction or terminal transition.
- The Store schema and Bundle format do not change.

## Non-goals

- Automatic stale detection, semantic similarity, transcript parsing, or LLM
  judgment inside WorkVCS.
- Automatic Record closure, correction, supersession, or Knowledge promotion.
- Treating every completed Task or Plan as evidence that related Records are
  no longer current.
- Requiring this audit for every task, Plan, Session, or closeout.
- Adding a generic Record-to-Plan ownership relation in this slice.
