# ProjectRef Journal Admission and Rollback Quiescence Evidence

Status: Implemented and concurrency-tested on isolated fixtures; no live
admission, migration, rollback, routing activation, installation, or commit
Date: 2026-09-25
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
Contract: [Registry v2 Migration and Acceptance](../architecture/projectref-registry-v2-migration-and-acceptance.md)

## Authorized boundary

This slice closes the source-level race between durable CaptureIntent admission
and the existing v2-to-v1 registry rollback candidate. All journal admission,
migration, rollback, contention, crash, and recovery mutations in the evidence
harness used disposable fixture directories, registries, and Stores. The
configured registry and retained Hernes Store were inspected read-only. No
source-tree binary was installed, no live routing or journal path was
activated, and no Git commit, push, release, deployment, or global Hook was
created.

The WorkVCS Goal/Plan and its completion records are ordinary governance state
for this repository. They are not ProjectRef migration, routing, journal, or
target-Store writes.

## One lock identity across aliases

ProjectRef-v2 routed journals derive the exclusive lock from the canonical,
existing registry path:

```text
<canonical-registry-path>.journal-quiescence.lock
```

The lock is not derived from a configured-home or registry-sidecar journal
root. `CaptureJournal::for_project_registry` therefore accepts only the
canonical registry plus a closed standard-home/registry-sidecar alias choice,
derives only a rollback-scanned journal root, and gives both aliases the same
lock path. It rejects a relative, missing, or noncanonical registry input and
rejects standard-home selection for a nonstandard registry filename. The
generic standalone-journal constructor remains explicit and uses its local
admission lock only for foundation tests; a registry-coupled journal refuses
plain `admit`.

## Critical sections and lock order

Admission validates the CaptureIntent before locking, then acquires the shared
lock, parses the registry as v2, and requires the exact revision and canonical
digest observed by the caller. Only after that mandatory core check may an
additional caller check further restrict routing/activation eligibility. A
no-op caller check cannot bypass registry identity validation, and core repeats
the exact registry identity read after the additional check so a mutating
callback also fails closed. Only after all checks succeed does admission create
the journal layout and perform the
idempotency scan plus immutable install. A failed post-lock check creates no
journal artifact.

Rollback acquires locks only in this order:

```text
project registry mutation lock -> journal quiescence lock
```

It holds both from the first activation/journal/receipt inspection through the
snapshot, exact-v1 candidate, final preflight, atomic replacement, directory
sync, and post-install verification. The read-only `--rollback-check` does not
acquire either lock. It reports the derived quiescence path and state; a
present unowned lock is a blocking issue, while rollback's own verified guard
is treated as `owned` rather than mistaken for external use.

## Concurrent winner evidence

Two deterministic fixture tests force the opposite winner orders:

- `journal_admission_first_persists_intent_and_forces_rollback_to_fail_closed`
  pauses admission after it owns the shared lock and has revalidated v2.
  Rollback first obtains its registry lock and waits. Admission then installs
  the immutable intent and releases; rollback acquires the shared lock, sees
  the non-empty home-root journal, writes no rollback snapshot, and leaves the
  exact v2 registry authoritative.
- `registry_rollback_first_restores_v1_and_admission_persists_nothing` pauses
  rollback while it owns both locks, starts admission through the sidecar
  alias, and verifies that no journal layout appears. Rollback restores exact
  v1 bytes. Admission then either acquires and fails its v2 post-lock check or
  times out fail-closed; the fixture separately verifies the v1 recheck path
  when contention timeout wins. No intent is persisted.

Thus the safe outcomes are asymmetric but exhaustive: admission can win and
make rollback ineligible, or rollback can win and make admission ineligible.
There is no fixture outcome in which both report success.

## Crash and owner-replacement behavior

The lock uses create-new installation and stores PID, creation time, and a
process-local monotonic nonce. A guard verifies exact owner bytes before
removal. `journal_quiescence_guard_never_removes_a_replacement_owner` removes a
first fixture lock, installs a second owner, and proves that dropping the old
guard preserves the replacement.

There is deliberately no age- or PID-based automatic stale-lock steal. An
orphan causes admission and rollback to fail closed, and rollback readiness
reports `journal_quiescence_lock_state=present` plus a blocking issue. The core
and CLI fixture tests remove only the exact known orphan after proving the
blocked state, then verify successful recovery. No live recovery command or
automatic cleanup policy is included in this slice.

## Validation evidence

Validation against the final candidate includes:

- six journal-focused core fixture tests, including alias convergence,
  post-lock-before-layout ordering, orphan recovery, and replacement-owner
  preservation;
- three CLI quiescence fixtures covering both race orders and orphan rollback
  recovery;
- the existing eight-test rollback filter, covering exact restore, re-entry,
  alias gating, pre/post-rename faults, and snapshot safety;
- `cargo test --workspace --all-targets`, including all 246 CLI tests and every
  core unit/integration target;
- strict workspace/all-target Clippy with warnings denied;
- schema v0.1 validation;
- full CLI smoke workflow with `smoke_result=passed`;
- operator recovery maturity with `PASS`, 51 core error codes, 52 guide
  entries, zero missing/extra coverage, and retryability matching the core
  rule; and
- source formatting plus `git diff --check`.

The first parallel full-suite run exposed a test assumption rather than a
safety failure: under load, waiting admission could reach its bounded lock
timeout before rollback released the lock. That outcome was already
fail-closed. The acceptance fixture now recognizes both safe rollback-first
outcomes and separately proves the post-rollback v2 revalidation path.

An independent read-only review found no blocker, but identified two
release-significant API gaps: callers could choose an arbitrary
registry-coupled journal root, and a caller-provided no-op post-lock callback
could bypass the intended eligibility check. The final candidate removes the
arbitrary-root parameter, derives the root from a closed alias enum, and makes
exact v2 revision/digest validation unconditional in core. The review also
recommended defending against a registry-mutating additional callback; the
final candidate repeats the identity read after that callback and tests the
failure path. The same reviewer then rechecked these closures after the focused
and full validation runs.

## Live non-operation baseline

Before and after this slice, the configured live registry remained v1 at
`/Users/example/.codex/workvcs/project-bindings.json`, with SHA-256
`c57ca8aa9b1f4a7d3ff0b1db9a35384b7e3776f809b7fb44737498a0ee323ed4`,
size `2862`, mode `0644`, and inode `107118324`. The retained Hernes Store at
`/Users/example/.codex/workvcs/stores/projects/<redacted-project-store>.sqlite`
retained SHA-256
`f3f2cdc3c8ed6f842ddf20ec9b81f89a440ccf6d72f354bcd691b350081cd644`,
size `712704`, mode `0644`, and inode `106869852`.

The live canonical-registry quiescence lock, both routing-activation aliases,
both journal aliases, rollback snapshots, and rollback temp patterns remained
absent.

## Closeout boundary

This evidence closes the shared-lock source candidate and isolated M-32/M-34
fixtures. It does not make journal admission live. The next bounded slice must
wire the actual adapter/routing path through `for_project_registry` and the
mandatory revision/digest post-lock check plus a separately restrictive
activation check, define its exact activation/disable surface, and run
end-to-end journal-first admission without target Store delivery. Any live
registry migration, read activation, journal activation, rollback,
installation, commit, push, release, deployment, Store delivery, or global
Hook remains a separate confirmation gate.
