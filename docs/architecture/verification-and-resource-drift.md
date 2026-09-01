# Verification and Resource Drift

This document defines the confirmed V1 contract for immutable Verification
judgments, Verification Requirements, Resource observations, and derived
applicability. It realizes [ADR-0001](../decisions/adr/0001-verification-applicability-and-resource-drift.md).

## Judgment, Evidence, and applicability

A Verification is one immutable judgment made for one target under a recorded
basis:

```text
Verification V-101
  target: T-18/AC-2/VR-1
  result: passed
  method: structured, open descriptor
  basis: Resource Basis + Work-State Basis
  evidenced_by: E-501
```

Re-verification creates another judgment. It does not update `V-101`, and it
does not automatically declare the earlier judgment superseded merely because
the method was repeated or time advanced. If one execution supports judgments
with different targets or results, it creates separate single-target
Verifications that may share one immutable Evidence object.

The immutable defining closure includes result, target, Verification Basis,
Evidence set, method, judgment-owned semantic state, and the defining
`verifies`/`evidenced_by` edges. Those edges use the common Relation
infrastructure but cannot be removed or redirected while retaining the same
Verification identity. Any closure change creates a new Verification.

The immutable historical result and current applicability are different:

```text
V-101.result = passed

applicability(V-101, Branch A) = applicable
applicability(V-101, Branch B) = stale
```

V1 applicability is a branch-sensitive Derived Projection with three outcomes:

- `applicable`: every declared basis component can be shown still applicable;
- `stale`: at least one declared basis component has deterministically relevant
  drift;
- `unknown`: no component is stale, but at least one cannot be compared or
  lacks sufficient basis.

The combination rule is conservative:

```text
any stale        -> stale
else any unknown -> unknown
else             -> applicable
```

`unknown` is neither a pass nor a failure. It cannot satisfy a mandatory
criterion.

## Verification Requirements and effective AC state

An Acceptance Criterion may define zero or more required Verification
Requirements. A Requirement:

- has stable AC-local identity such as `T-18/AC-2/VR-1`;
- states what must be proven rather than a command or framework;
- remains historically referential when revised, retired, or superseded;
- is the direct Verification target when Requirements exist.

Without Requirements, one applicable passed Verification may directly satisfy
the AC. With required Requirements, every required Requirement must be
satisfied. V1 does not introduce a general policy DSL.

The effective AC projection is:

- `unverified`: no applicable judgment establishes the required result;
- `verified`: all required coverage is satisfied by applicable passed
  judgments and no unresolved applicable failure defeats it;
- `failed`: an applicable failed judgment blocks the criterion under the
  confirmed rule;
- `stale`: historical judgments exist but no longer establish the current
  criterion because their applicability is stale or unknown;
- `conflicted`: applicable passed and failed judgments cannot be resolved
  deterministically.

Task completion requires every mandatory AC to project to `verified`. A
coordination force or claim takeover cannot bypass this semantic gate. A future
exception would require its own confirmed semantic operation and provenance.

## Verification Basis

The Basis states what the judgment depended on. It may contain:

```text
Resource Basis
  resource identity
  declared scope
  baseline ResourceObservation and/or fingerprint

Work-State Basis
  verified_at_workstate
  explicit semantic dependencies
```

Resource scope supports at least:

- the whole logical Resource;
- a path-scoped subset;
- an explicit artifact/input set.

High precision is not mandatory for every judgment. Missing or incomparable
basis produces `unknown`, rather than a guessed answer. Work-State applicability
compares the recorded dependencies from `verified_at_workstate` to the current
Branch state; a different Branch head alone is not sufficient to mark a
judgment stale.

## Resource and ResourceObservation

A Resource has stable logical identity, kind, and an environment-specific
locator binding. Logical Resource and immutable kind are ObjectIdentity-backed;
the current locator belongs to a separate environment-local ResourceBinding.
Moving a Store or rebinding the locator preserves logical
identity but does not prove that the new binding contains continuous content.
Continuity still requires a matching fingerprint or another adapter-proven
source/content relationship.

A ResourceObservation is immutable provenance about a Resource at a point in
time. Verification persists the observation and/or fingerprint used as its
basis. Other query-time observations may remain ephemeral unless an explicit
snapshot or later policy requires persistence.

WorkVCS Core does not interpret Git, directories, or files directly. It asks a
Resource Adapter to:

- describe/observe a Resource;
- produce a deterministic fingerprint for a declared scope;
- report the deterministic difference between baseline and current state for
  that scope.

An adapter must define deterministic scope matching and normalization. The
exact glob, path, symlink, case, rename, and deletion rules remain an
implementation specification, not a confirmed choice here.

For Git-backed Resources, an observation used for Verification must represent
the actually verified state, including relevant index or working-tree changes;
HEAD alone is insufficient. If a locator is missing or comparison cannot be
performed, the Resource component is `unknown`, not assumed unchanged and not
automatically stale.

## Mechanical drift versus semantic cognition

Creating an observation or detecting drift is not a Work-State semantic
mutation:

```text
Resource change
  -> ResourceObservation or ephemeral observation
  -> derived drift/applicability
  -> no WorkStateCommit
```

If an Agent decides the drift matters semantically, it explicitly records a
Finding, re-verifies, reopens a Task, or changes a Decision. Only that semantic
operation creates a WorkStateCommit.

## Current implementation boundary

Adapter and scope contract versions are persisted, while WorkVCS hashes their
canonical normalized bytes with the Store digest algorithm (BLAKE3-256 in V1).
Verification target/Evidence keep one Relation authority and Applicability
uses the confirmed physical constraints in
[Physical Schema v0.1 Contract](physical-schema-v0.1.md). ADR-0444 adds the
first executable adapter-backed refresh contract: exact `local-file` path
Resource basis entries can be re-observed through
`verification cache-refresh --resource-content-from-scope-path`. Final CLI
names, broader adapter implementations, glob/path-prefix semantics, symlink,
case, rename and deletion policy, non-Verification observation capture policy,
performance indexes, and a possible future AC waiver operation remain unfixed.
The executable schema, Rust implementation language, `rusqlite`, and the
`workvcs-jcs-v1` canonical JSON profile are now closed by accepted
implementation ADRs.
