# ProjectRef Primary Delivery and Receipt Recovery Candidate Evidence

Status: Implemented and validated on isolated fixture Stores; no live
registry, journal, or Store mutation, activation, installation, or commit
Date: 2026-09-27
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
Contract: [ProjectRef Control Plane v2](../architecture/projectref-control-plane-v2.md)
Acceptance: [Registry v2 Migration and Acceptance](../architecture/projectref-registry-v2-migration-and-acceptance.md)
Roadmap position: round 3 of 11 complete; eight mainline rounds remain

## Authorized boundary

This round begins only after a CaptureIntent, current resolution, and exact
ProjectRef binding are durable. It extends the separately explicit
`project capture-recovery --apply` path through one canonical target delivery
and a complete immutable receipt. Every ProjectRef registry, capture journal,
projection, semantic target Store, fault, and recovery mutation used
disposable fixture roots.

The WorkVCS Goal, Plan, Tasks, verification Evidence, and completion records
written to the repository's already-bound governance Store are ordinary
governance state for this implementation round. They are not ProjectRef
registry, routed capture-journal, activation, or semantic target-Store writes.

The ordinary capture route remains admission-only. Neither activation marker
starts recovery or Store delivery. This round does not implement secondary
references, activate routing, install a binary, change the configured live
registry or its journal aliases, create a Git commit, push, release, deploy, or
install a global Hook.

## Guarded primary delivery

Before opening the target Store for mutation, recovery materializes the
target-neutral v2 semantic payload into the existing cognition-capture
manifest and records `delivery_started`. That event fixes:

- one delivery ID and `canonical` mode;
- the exact ProjectRef, Store, Workspace, and Branch;
- expected Branch head and state digests;
- an idempotency key derived from the Capture ID plus the complete target
  tuple; and
- the materialized manifest digest.

The target is verified read-only, then its Store/Workspace/Branch identity is
checked again after writable open. A CaptureGroup must name a real Record in
the materialized manifest before the Store is opened for mutation. Invalid
event transitions are folded and rejected before an immutable event file is
installed.

The write itself reuses the existing atomic, idempotent
`cognition.capture` engine. `delivery_applied` records the resulting
WorkStateCommit, ChangeSet, state digest, all Record/Knowledge/Evidence/
Relation identities and immutable-version digests, target reuse outcome, and,
for a CaptureGroup, the canonical Record reference. Result kinds and typed IDs
are validated; duplicate local or immutable results fail closed.

## Commit-before-receipt recovery

If the target commit succeeds before `delivery_applied` becomes durable, the
journal remains `pending_primary` with the original `delivery_started` event.
Retry reconstructs the exact manifest and target idempotency identity from
that event. The Store returns the original result with `reused=true`, and
recovery installs only the missing receipt. It does not create a second
commit, Record, Knowledge, Evidence, or relation.

Receipt history survives a registry metadata refresh only while resolution
still names the same ProjectRef. It is inactive at `pending_project` until a
new binding receipt revalidates the exact Store/Workspace/Branch tuple. An
exact match restores the completed receipt without a target call. A target
change clears the current delivery view and returns to `pending_primary`
without writing either Store.

## Legacy and reference boundaries

A retained `legacy_cognition_v1` manifest is not silently rebased. If its
explicit head/state guards are absent or stale, recovery records
`delivery_failed` with `legacy_manifest_upgrade_required`; the admitted intent
remains byte-identical and the target Store remains unchanged.

A successful primary receipt for a CaptureGroup derives
`pending_references`. Round 3 exposes the canonical Record reference needed by
round 4 but does not write a secondary association or claim cross-Store
atomicity.

## Fault and fixture evidence

The CLI fixture matrix injects failures after Store bootstrap, registry temp
sync, registry replacement, registry-directory sync, binding-event install,
delivery-start install, target commit, delivery-receipt install, and projection
replacement. Pre-authority failures remain safe; post-install uncertainty
requires status-first forward recovery. All nine windows converge to one
ProjectRef/binding and exactly one semantic target commit.

Focused core coverage includes:

- `commit_before_receipt_recovery_reuses_one_primary_result`;
- `primary_receipt_exposes_canonical_record_and_waits_for_references`;
- `stale_legacy_manifest_is_detected_without_rewriting_the_intent`;
- `registry_revalidation_preserves_receipt_only_for_the_exact_target`;
- `invalid_delivery_transition_is_rejected_before_event_install`; and
- `capture_group_canonical_record_is_preflighted_before_target_write`.

Focused CLI coverage includes:

- `cli_capture_recovery_delivers_primary_once_and_reuses_receipt`;
- `capture_recovery_faults_converge_forward_without_duplicate_identity`;
- `stale_legacy_manifest_requires_upgrade_without_target_store_write`; and
- `capture_recovery_conflict_records_status_without_bootstrap_or_fallback`.

Together these close the implemented parts of C-05, C-13, C-14, C-18,
C-21, C-23, C-24, C-25, M-10, and M-11 without claiming secondary delivery or
live operation.

## Validation evidence

Validation against the final source candidate passed with the local LLVM
toolchain:

- `cargo test --workspace --all-targets --quiet`, including all 253 CLI tests
  and every core unit/integration target;
- the six focused core primary-delivery tests and four focused CLI recovery
  tests;
- strict workspace/all-target Clippy with warnings denied;
- schema v0.1 validation;
- the full CLI smoke workflow with `smoke_result=passed`;
- operator recovery maturity with `PASS`, 53 core error codes, 54 guide
  entries, zero missing/extra coverage, and retryability matching the core
  rule;
- Rust formatting and documentation-link checks; and
- `git diff --check`.

Independent read-only review found and closed four material edge cases before
this evidence was finalized: registry refresh could discard a valid receipt;
an invalid transition could be installed before projection folding; a
CaptureGroup canonical local ID could fail only after the target commit; and a
target path could be replaced between binding verification and writable open.
The exact-target retention rule, pre-install fold, preflight, repeated target
identity checks, and focused regressions now cover those cases.

## Live zero-write proof

Before implementation and again after final validation, the configured
registry remained v1 at
`/Users/example/.codex/workvcs/project-bindings.json`, with SHA-256
`c57ca8aa9b1f4a7d3ff0b1db9a35384b7e3776f809b7fb44737498a0ee323ed4`, size
`2862`, mtime `1789695294`, inode `107118324`, and mode `0644`.

Both home and registry-sidecar read-routing markers, journal-admission
markers, capture-journal aliases, and the canonical journal-quiescence lock
were absent at both probes. The source binary was never pointed at those
paths.

## Next confirmation gate

Round 4 was later completed under the same isolation boundary; see
[ProjectRef CaptureGroup Secondary Reference Evidence](projectref-capture-group-secondary-reference-candidate.md).
The next confirmation gate is round 5: one concrete tool adapter behind the
generic interface, WorkVCS Skill ordering, the full acceptance matrix,
operator material, independent review, and a refreshed zero-write real-state
preview. It must still exclude real registry/journal/Store mutation,
activation, installation, Git commit, push, release, deployment, and global
Hook work.
