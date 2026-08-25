# WorkVCS Documentation

This directory is the repository-native source of truth for WorkVCS's
confirmed product, domain, and architecture state.

## Current baseline

The confirmed baseline contains:

- [Product definition](product/product-definition.md)
- [V1 and V2 boundary](product/v1-v2-boundary.md)
- [Domain entities](domain/entities.md)
- [Typed relationships](domain/relationships.md)
- [Domain invariants](domain/invariants.md)
- [System boundaries](architecture/system-boundaries.md)
- [Versioning engine](architecture/versioning-engine.md)

These documents specify what WorkVCS currently means. They do not claim that
the described runtime has been implemented.

Detailed promotion evidence and the confirmation ledger are retained in
[Confirmed State v0.1 Provenance](provenance/confirmed-state-v0.1.md). That
document supports audit and reconstruction but is not a parallel normative
specification.

## Authority and conflict handling

For repository work, follow [`AGENTS.md`](../AGENTS.md). Its workflow and ADR
requirements are repository-governance policy; they are not retroactively
classified as product decisions confirmed in the source conversation.

Within the confirmed documentation set, later explicit user confirmation
supersedes an older decision in the affected scope. Domain invariants constrain
architecture, architecture realizes the confirmed domain, and product documents
define value and release scope. An ADR is a recording and explanation vehicle:
it becomes product authority only when the exact decision it contains has been
accepted. Writing an ADR does not promote an unconfirmed proposal by itself.

If two documents appear to conflict, classify the disputed point as Open or
Unclear, trace both interpretations to their evidence, and obtain an explicit
decision before implementation chooses between them.

## State classification

- **Confirmed:** directly stated by the user or contained in an exact option
  the user explicitly accepted.
- **Open:** a current decision is still required. Open material cannot redefine
  confirmed behavior.
- **Rejected/Superseded:** explicitly rejected or replaced by a later confirmed
  decision. It is retained only as provenance, not current state.
- **Unclear:** the available evidence cannot determine the current answer. Do
  not infer one for implementation convenience.

Generated indexes, summaries, and runtime projections are derived artifacts,
not an additional decision state or an independent authority.

## Updating the baseline

A change to confirmed design must identify its source, update every affected
document, check the invariants, and receive validation at the same impact level.
Current repository policy may require a material architecture decision to be
recorded as an ADR, but the ADR is accepted only after its exact decision is
confirmed. Conversation history is supporting evidence, not a substitute for
this repository baseline.
