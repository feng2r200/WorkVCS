# ADR-0508: Record Bundle Same-Store Apply

Status: Accepted
Date: 2026-09-12

## Context

The Question/Risk currentness dogfood exported a valid Bundle but could not
apply it to an earlier copy of the same Store. The Bundle apply capability
check accepted Task, verification, and Knowledge entity families but rejected
the generic `record` entity kind. This meant Findings, Decisions, Questions,
Risks, Attempts, Assumptions, and Handoffs were portable as validated payloads
but not recoverable through the supported same-Store fast-forward path.

That boundary conflicts with WorkVCS's role as portable durable cognition. It
also prevented an exact round-trip check for the new Question/Risk terminal
states.

## Decision

The existing same-Store Bundle apply profile accepts the `record` entity kind.
Record versions continue to use the generic immutable EntityVersion and
ChangeOperation paths; no Record-specific tables or import semantics are
introduced.

The supported profile remains a same-identity, existing-Branch fast-forward.
This change does not create missing Branches or enable external-Store canonical
DAG activation.

## Consequences

- Record history, including terminal Question/Risk states, can be restored to
  an earlier copy of the same Store.
- The existing payload, provenance, branch compare-and-swap, and integrity
  checks remain authoritative.
- A focused regression proves four missing Record commits apply and preserve
  `answered` and `mitigated` at the imported head.
- Other entity families remain outside apply until evidence justifies their
  admission; this change does not turn an allow-list into an unchecked generic
  import.

## Non-goals

- Different-Store import or merge.
- Automatic Record conflict resolution.
- New Record kinds, relations, or lifecycle states.
- General Goal/Plan portability in this slice.
