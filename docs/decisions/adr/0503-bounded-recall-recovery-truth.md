# ADR-0503: Bounded Recall Recovery Truth

Status: Accepted
Date: 2026-09-12

## Context

`recall retrospective` already samples semantic categories, but `brief` and
`handoff` append Records in oldest-first order. With a small budget, historical
findings and already-remediated risks can hide the current decisions and
evidence. Retrospective category reservation can also consume the whole budget
before any Goal, Plan, or Task appears.

An isolated read-only Codex reconstruction reproduced the practical cost. It
raised the budget to 200 and then issued many object-by-object queries to recover
Goal/Plan/Task, acceptance, evidence, relations, and history. The same probe
exposed two distinct closeout-currentness defects: an active Claim was omitted
when its Session had no focus, and branch-based authorization receipts were
evaluated at the latest Work-State commit timestamp instead of read time.
Session and Claim transitions intentionally do not move the Work-State head, so
live coordination must not be inferred from that timestamp.

The first implementation then exposed a second ordering trap during dogfood:
snapshot `commit_id` is the commit at which the object was evaluated, so every
current object can carry the same Branch head. It is not the object's creation
or last-transition commit and cannot be used as a newest-first sort key.

## Decision

Recall is a bounded recovery packet, not a chronological dump:

- one representative from each non-empty active Goal, active Plan, live
  Session, active Claim, and non-terminal Task category is reserved before any
  category can consume the remaining budget; live coordination precedes the
  Task reserve because a Claim already identifies its owned Task;
- live Runtime Coordination includes both active and potentially stale Sessions
  together with their active Claims, because potentially stale is recoverable,
  not ended;
- semantic Records, Knowledge, and relations are ordered newest-first by their
  current version identifiers; Evidence uses capture time. Snapshot evaluation
  commits are labelled as such and are not mistaken for version creation time;
- retrospective reserves both current category representatives and
  representative semantic categories before filling the remaining budget;
- Record scope and Knowledge scope/provenance are rendered so source identifiers
  and applicability can be checked without an immediate `show` round trip;
- the projection states whether each category is current active work or
  historical/terminal inventory rather than asking consumers to infer that from
  position alone; and
- branch-based closeout evaluates Runtime Coordination at read time. An explicit
  historical commit remains a historical projection and is labelled accordingly.

Default budgets must be sufficient for representatives of the current work
categories plus recent reasoning in ordinary cases. A caller may deliberately
expand the budget to enumerate a large Task queue or run a deep retrospective,
but correctness of current-state reconstruction must not depend on requesting
the maximum.

## Consequences

- Fresh models can reconstruct the current objective, route, next executable
  work, recent decisions/risks, and supporting identifiers without scanning the
  Store.
- Historical findings remain available but no longer masquerade as the first
  current context merely because they were created earliest.
- Live Runtime Coordination is not silently hidden by a Work-State timestamp
  that Runtime operations intentionally do not advance.
- A large Task queue cannot hide all current ownership detail; category totals
  still make any bounded omissions explicit.
- Raw Evidence bodies remain an independent bundle-portability concern; this ADR
  changes projection and currentness, not Evidence export format.
