# Phase 5G Record Currentness Audit Evidence

Status: current local evidence
Date: 2026-09-13

## Problem

WorkVCS preserved the reasoning history needed for retrospectives, but current
reads could still present explicitly unresolved obligations together with
claims whose work had already been completed. Recovering the true next action
therefore required broad Recall, manual Record queries, and knowledge of which
older statements had been superseded.

The required correction was not an automatic stale-record detector. Whether a
statement is obsolete is a semantic judgment, and independent Records must not
silently become Plan closeout blockers.

## Implemented boundary

`workvcs record currentness-audit` is a bounded, read-only review surface over
the selected Work-State.

By default it reports explicit open obligations:

- `unverified` Assumptions;
- `running` Attempts;
- `active` Questions; and
- `active` Risks.

`--include-current-claims` additionally includes validated Assumptions and
active Decisions and Findings. `--kind`, exact `--scope-json`, and
`--statement-contains` narrow the review set. The default item budget is 50 and
the hard maximum is 200; results are ordered by newest current Record version
and report totals, omissions, and truncation explicitly.

Every candidate preserves its full statement and scope, stable Record and
version identities, digest, lifecycle state, allowed explicit outcomes, and a
review command. Output also states:

```text
audit_basis=explicit_lifecycle_state
automatic_stale_inference=false
plan_closeout_blocking=false
requires_explicit_judgment=true
read_only=true
```

A current Branch head is marked potentially mutation-eligible. Historical
Commit inspection is marked `source_mutation_eligible=false` and offers only
inspection or comparison to current state. The query uses the verified
read-only Store/Engine path and performs no migration or repair.

The accepted semantic contract is recorded in
[`ADR-0510`](../decisions/adr/0510-record-currentness-audit.md).

## Validation and installation

Implementation commit
`f676485eba39e43350d6118b529b26969e79b69d` passed:

- the focused core currentness-audit tests;
- the CLI audit and nested-help regressions;
- `cargo test --workspace --quiet`, including all 201 CLI tests;
- workspace all-target Clippy with the repository's pre-existing
  argument-count lint excluded;
- formatting and diff checks; and
- validation of the complete five-file WorkVCS Skill tree.

The exact clean commit was packaged and installed to
`/Users/example/.local/bin/workvcs` and `/Users/example/.agents/skills/workvcs`.
The installed binary SHA-256 is
`1540c4109080b75484bad9c5b54764e521749bfc249966b01378df02dd3c2ada`;
the installed Skill tree SHA-256 is
`f4a890278acd51421d357566a51c26d680999ca9ede4801f1d58dcba1802304e`.
Installed help, current and historical audit probes, and required-valid Store
doctor checks passed.

## Maintained-Store dogfood

The first installed audit reported one active Question and seven active Risks.
Explicit review then:

- answered the audit-design Question;
- mitigated six Risks whose cited implementation or validation now exists;
- kept one genuine Risk active: globally loaded WorkVCS guidance can recreate
  mandatory calls, Plan coupling, context cost, or interruption if loading the
  guidance is confused with deciding that a concrete operation has value;
- created one authoritative current-state Finding and used it to supersede nine
  current-tense Findings contradicted by completed implementation or validation;
  and
- created one evidence-driven next-work Decision and used it to supersede two
  completed priority queues.

No Record was deleted. Terminal versions and `supersedes` relations remain in
history for retrospective reconstruction.

After reconciliation, the default audit returns exactly one open obligation.
The expanded 200-item audit returns 33 candidates: 11 active Decisions, 21
active Findings, and the retained Risk, with no omissions or truncation. Exact
filters for both completed priority statements return zero current candidates.

## Decision

Do not add another WorkVCS surface speculatively. Keep the remaining guidance
Risk visible and choose the next optimization only when current dogfood or
project evidence demonstrates concrete friction, incorrectness, or missing
retrospective value.

## Boundary

The command inventories explicit lifecycle state; it does not infer truth,
importance, age-based staleness, or semantic contradiction. A model or human
must inspect the candidate and choose `retain` or an explicit guarded terminal
transition. The audit does not make Plan mandatory, does not create Plan gaps,
and does not authorize mutation merely because a candidate was displayed.
