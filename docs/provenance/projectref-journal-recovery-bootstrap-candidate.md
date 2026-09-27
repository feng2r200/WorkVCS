# ProjectRef Journal Recovery and Bootstrap Candidate Evidence

Status: Implemented and validated on isolated fixtures; no semantic target
delivery, live registry mutation, activation, installation, or commit
Date: 2026-09-27
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
Contract: [ProjectRef Control Plane v2](../architecture/projectref-control-plane-v2.md)
Acceptance: [Registry v2 Migration and Acceptance](../architecture/projectref-registry-v2-migration-and-acceptance.md)
Roadmap position: round 2 of 11 complete; nine mainline rounds remain

## Authorized boundary

This round starts only after an immutable CaptureIntent already exists. It
adds immutable recovery events, deterministic rebuildable projections,
read-only recovery status, and explicit digest-locked ProjectRef/binding
convergence. Every registry, journal, projection, Store, fault, and recovery
mutation used disposable fixture roots. No command from this checkout was run
against the configured live registry.

This round does not deliver the admitted manifest into a target Store. A newly
initialized Store remains at pristine Genesis with its default Workspace and
Branch. No Record, Knowledge, Evidence, relation, CaptureGroup delivery,
secondary reference, Git commit, installation, live marker, push, release,
deployment, or global Hook was created.

## Immutable authority and rebuildable projection

Post-intent events are canonically encoded immutable files. Each envelope
contains a capture ID, contiguous sequence, event ID, event time, payload
digest, and previous-event digest. Filename identity, canonical bytes, payload
digest, sequence, and chain continuity are all validated. A gap, reordering,
rename, payload drift, symlink, or broken chain fails closed.

The capture projection is derived only from the immutable intent and event
chain. Deleting it or replacing it with stale or malformed regular bytes does
not destroy authority: deterministic replay recreates the same canonical
bytes. Repeating an already-current resolution or binding-ready transition
reuses the latest matching event instead of adding a duplicate. Projection
repair never suppresses event corruption.

## Explicit status and convergence

The source-tree CLI exposes:

```text
workvcs project capture-recovery --status --capture-id ID [--registry PATH]
workvcs project capture-recovery --apply --capture-id ID --expected-registry-digest DIGEST --expected-projection-digest DIGEST [--registry PATH] [--store-root PATH]
```

Status is read-only. It locates the closed home/sidecar journal alias,
validates authority, derives the projection, reports stored projection state,
re-resolves current ownership, validates any resolved target, and gives the
exact next action. Duplicate aliases fail closed.

Apply is a separate Unix-only operator action and is not enabled by either
activation marker. It requires the exact status-observed registry and
projection digests before and after acquiring locks in this order:

```text
registry mutation lock -> shared journal quiescence lock -> capture event lock
```

An unbound semantic or Git owner converges to one established ProjectRef,
locator, binding, and pristine Store. CWD-only ownership converges to one
provisional ProjectRef. A matching existing locator/target is reused; target
substitution fails closed. Conflict or unresolved ownership records only the
pending resolution state and never falls through to a repository, CWD, new
ProjectRef, or Store.

The original migration receipt remains scoped to ProjectRefs created by the
migration. A post-migration first-write ProjectRef therefore does not invent a
mapping row or invalidate the receipt.

## Fault and recovery contract

The isolated CLI harness injects failures after Store bootstrap, registry temp
sync, registry replacement, registry-directory sync, binding-event install,
and projection replacement. Failures before authoritative registry install
are safe failures. Failures after a registry/event/projection install return
`capture_recovery_install_indeterminate`.

Recovery never guesses an inverse operation. The operator starts again with
read-only status, observes the actual registry/event/projection state, takes
fresh digests, and converges forward. Across all six boundaries, replay creates
no duplicate ProjectRef, locator, binding, event, Store, Workspace, Branch, or
semantic object. Candidate temp files are removed, and previously existing
Stores remain byte-stable.

## Fixture evidence

The round adds these focused tests:

- `projection_rebuild_is_byte_equivalent_and_replay_has_no_duplicates`;
- `event_chain_gaps_reordering_and_payload_tampering_fail_closed`;
- `first_write_binding_converges_once_and_rejects_target_substitution`;
- `repository_and_cwd_bootstrap_preserve_maturity_provenance`;
- `cli_capture_recovery_bootstraps_once_without_semantic_store_delivery`;
- `capture_recovery_faults_converge_forward_without_duplicate_identity`; and
- `capture_recovery_conflict_records_status_without_bootstrap_or_fallback`.

Together they prove C-18 through C-23 and M-39 in the acceptance contract.
They also preserve the prior migration, rollback, activation, admission, and
quiescence fail-closed contracts.

## Validation evidence

Validation against the final source candidate passed with the local LLVM
toolchain:

- `cargo test --workspace --all-targets --quiet`, including all 252 CLI tests
  and every core unit/integration target;
- all four focused core journal-recovery tests and all three focused CLI
  capture-recovery tests;
- strict workspace/all-target Clippy with warnings denied;
- schema v0.1 validation;
- the full CLI smoke workflow with `smoke_result=passed`;
- operator recovery maturity with `PASS`, 53 core error codes, 54 guide
  entries, zero missing/extra coverage, and retryability matching the core
  rule;
- Rust formatting and the new documentation-link existence checks; and
- `git diff --check`.

Implementation review also identified one projection-fold edge case before
closeout: a later resolution change could leave an earlier binding-ready
receipt looking current. The final fold clears the derived binding receipt on
every new resolution event and accepts it again only after a matching later
binding-ready event. The byte-rebuild fixture now covers that degradation and
recovery path.

## Live zero-write proof

Before implementation and again after final validation, the configured
registry remained v1 at
`/Users/example/.codex/workvcs/project-bindings.json`, with SHA-256
`c57ca8aa9b1f4a7d3ff0b1db9a35384b7e3776f809b7fb44737498a0ee323ed4`, size
`2862`, mtime `1789695294`, inode `107118324`, and mode `0644`.

Both home and registry-sidecar read-routing markers, journal-admission
markers, capture-journal aliases, and the canonical journal-quiescence lock
were absent at both probes. The source binary was never pointed at those paths.

## Next confirmation gate

Round 3 may implement primary target Store delivery and receipt recovery only
on disposable fixture Stores. It must reuse the admitted semantic payload and
target idempotency identity, distinguish a committed target from a missing
receipt, and prove crash recovery without duplicate semantic objects. It must
still exclude live registry/journal/Store mutation, installation, activation,
commit, push, release, deployment, secondary-reference delivery, and global
Hook work.
