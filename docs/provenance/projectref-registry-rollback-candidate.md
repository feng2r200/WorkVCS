# ProjectRef Registry Rollback Candidate Evidence

Status: Implemented and fault-tested on isolated fixtures; no live rollback,
migration, routing activation, installation, or commit
Date: 2026-09-24
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
Contract: [Registry v2 Migration and Acceptance](../architecture/projectref-registry-v2-migration-and-acceptance.md)

## Authorized boundary

This slice implements the separately explicit rollback candidate and its
read-only recovery probe. Every mutating invocation in the evidence harness
used temporary fixture registries and Stores. The configured registry and its
retained Hernes Store were inspected read-only. The source-tree binary was not
installed, the live registry was not migrated or rolled back, routing was not
activated, and no Git commit, push, release, deployment, or global Hook was
created.

The command surface is:

```text
workvcs project registry-migrate --rollback-check \
  --expected-installed-digest DIGEST \
  --expected-backup-digest DIGEST [--registry PATH]

workvcs project registry-migrate --rollback \
  --expected-installed-digest DIGEST \
  --expected-backup-digest DIGEST [--registry PATH]
```

Neither action accepts an ownership-repair manifest or non-text output.
`--rollback-check` is always read-only. `--rollback` is a mutation and requires
separate authority for the exact target even when the probe reports ready.

## Locked state and zero-use proof

For an installed v2 registry, the shared inspection path requires:

- canonical stored v2 bytes whose digest equals the expected installed digest;
- revision `1` and the exact migration receipt;
- receipt source, preview, backup path, backup digest, and mapping coverage;
- exact regular, non-symlink v1 backup bytes with the expected raw digest and
  receipt source digest;
- successful read-only verification of every binding target;
- absence across every home-root and registry-sidecar alias of the
  digest-bound read-routing activation marker; and
- every supported capture-journal `intents`, `events`, `projections`, and
  `locks` directory to be absent or empty.

For a standard-name registry, the probe checks both direct-registry sidecar and
sibling WorkVCS-home marker/journal layouts regardless of whether the call used
configured home or explicit `--registry`. Canonically equivalent home paths are
deduplicated. A malformed, unreadable, symlinked, non-empty, stale, or
conflicting artifact fails closed.

The empty-journal observation is not claimed as a future concurrency barrier.
It is admissible in this slice because v2 journal admission and delivery remain
disabled. Before either route is enabled, admission and rollback must share one
quiescence lock across every alias and pass a concurrent race test; otherwise
rollback remains disabled.

## Atomic rollback sequence

While holding the existing cross-process registry lock, rollback:

1. performs the complete shared read-only inspection;
2. writes the exact installed v2 bytes to a same-directory temp, syncs and
   reparses them, then atomically installs the immutable digest-named snapshot
   `<registry>.v2.<installed-digest>.rollback.bak` without overwrite;
3. writes the exact verified v1 backup bytes to a same-directory candidate,
   restores the original v1 permissions, syncs, rereads, and reparses it;
4. reruns every digest, receipt, activation, journal, Store, artifact, and
   byte-stability precondition immediately before replacement;
5. atomically renames the candidate over the registry and syncs the parent;
6. verifies the exact current v1 bytes, canonical source digest, retained v1
   backup, retained v2 snapshot, binding targets, and zero-use state before
   reporting success.

Apply and rollback currently require Unix-family same-filesystem
rename-over-existing semantics. A non-Unix invocation fails before lock or
write until a platform-specific replace primitive and fault matrix are
accepted.

The v2 snapshot is recovery evidence, not a second live registry. Ordinary
commands never route through it. An existing expected snapshot is reused only
when it contains the exact canonical stored v2 bytes and matching backup
receipt. Another digest-named snapshot or any stale registry temp blocks the
operation.

## Failure and re-entry semantics

Failures before the registry rename emit `registry_rollback_failed`. The
authoritative registry is not replaced, exact temp files are removed, and a
fully installed exact v2 snapshot may remain reusable. Failures after rename
emit `registry_rollback_install_indeterminate`; the command never claims that
rollback failed or succeeded and directs the operator to the read-only probe.

The probe reports one of:

- `v2_ready`: exact v2 plus v1 backup and zero-use conditions are ready for a
  separately authorized rollback;
- `v1_restored`: current v1 bytes exactly equal the retained v1 backup and the
  exact v2 snapshot/receipt/digests close the recovery chain; or
- `blocked`: at least one digest, receipt, target, activation, journal, file,
  artifact, or stability check failed.

The machine-readable booleans remain deliberately distinct:
`rollback_apply_safe=true` only for `v2_ready`, and
`rollback_reentry_safe=true` only for `v1_restored`.

Repeating explicit rollback after `v1_restored` is a verified no-op. It writes
neither registry, backup, snapshot, journal, nor Store. A v1 registry without
the exact v2 snapshot and receipt chain is never guessed to be a completed
rollback.

## Fault-injection evidence

The isolated harness injected failures at every durable boundary:

| Boundary | Observed invariant |
| --- | --- |
| after v2 snapshot temp write | v2 remained exact; temp removed; no final snapshot |
| after exact v2 snapshot installation | v2 remained exact; snapshot retained and reusable |
| after v1 candidate temp write | v2 and both backups remained exact; candidate removed |
| after v1 candidate sync | v2 and both backups remained exact; candidate removed |
| immediately before registry rename | v2 remained authoritative; candidate removed |
| immediately after registry rename | exact v1 bytes were present; result was indeterminate |
| after parent-directory sync | exact v1 bytes were present; result was indeterminate |
| before post-install verification | exact v1 bytes were present; result was indeterminate |

The post-rename cases were recovered by `--rollback-check`, which observed
`v1_restored`; a subsequent explicit repeat returned a no-write reuse result.
Additional fixtures blocked rollback for an active marker, either journal
layout, configured-home/explicit-registry alias inversion, wrong installed or
backup digest, byte drift, noncanonical snapshot, snapshot symlink,
conflicting snapshot, and stale temp artifact. No fixture fault changed its
target Store.

## Validation evidence

Validation against the final implementation shape completed successfully:

- rollback-focused CLI tests: 7 passed;
- configured-home activation plus explicit-registry rollback alias test: 1
  passed;
- `cargo test --workspace --all-targets`: passed, including all 243 CLI tests
  and every core unit/integration target;
- strict workspace/all-target Clippy with warnings denied: passed;
- schema v0.1 validation: passed;
- full CLI smoke workflow: `smoke_result=passed`;
- operator recovery maturity: `PASS`, with 51 core error codes, 52 guide
  entries, zero missing/extra coverage, and retryability matching the core
  rule; and
- source formatting plus `git diff --check`: passed.

The independent read-only review first identified the activation-path alias
gap, future journal-admission race, cross-platform replace overclaim, and
ambiguous re-entry flag. Follow-up review confirmed that the marker/journal
aliases and distinct state flags are covered by code and fixtures, Unix-only
replacement now fails closed before lock/write, and the journal race is
accurately retained as a release gate rather than claimed solved. It reported
no new safety regression. The reviewer did not execute tests; the test results
above are the primary validation run.

## Live non-operation baseline

The configured live registry remained v1 at
`/Users/example/.codex/workvcs/project-bindings.json`, with SHA-256
`c57ca8aa9b1f4a7d3ff0b1db9a35384b7e3776f809b7fb44737498a0ee323ed4`,
size `2862`, mode `0644`, and inode `107118324`. The retained Hernes target
Store remained at
`/Users/example/.codex/workvcs/stores/projects/<redacted-project-store>.sqlite`,
with SHA-256
`f3f2cdc3c8ed6f842ddf20ec9b81f89a440ccf6d72f354bcd691b350081cd644`,
size `712704`, mode `0644`, and inode `106869852`. No live routing activation
marker, v2 rollback snapshot, or rollback temp existed. WorkVCS governance
records for this implementation Plan are the only authorized non-fixture
Store writes and are not migration, rollback, routing, or target-delivery
operations.

## Closeout

This isolated-fixture candidate round is complete within its authorized
boundary. Exact byte restoration, digest locking, backup/snapshot reuse,
pre/post-rename fault classification, alias-aware activation/journal gating,
and verified no-write re-entry all have executable evidence. The configured
live registry and retained Hernes Store match the baseline hashes, sizes,
modes, and inodes above, and both live marker/journal aliases plus rollback
snapshot/temp patterns remain absent.

This is not release or live-operation authorization. The next bounded round is
the journal-admission/rollback quiescence design and concurrent race fixture.
It must keep durable admission disabled until one shared lock makes admission
and rollback mutually exclusive across every alias. Non-Unix atomic replace
support remains a separate platform-specific design and fault-validation
gate. Live migration, read activation, rollback, installation, commit, push,
release, and global Hook activation remain outside this closeout.

Follow-up on 2026-09-25: that isolated shared-lock round is now complete; see
[ProjectRef Journal Admission and Rollback Quiescence Evidence](projectref-journal-rollback-quiescence-candidate.md).
Durable admission and every live/installation/Git delivery boundary remain
disabled as described there.
