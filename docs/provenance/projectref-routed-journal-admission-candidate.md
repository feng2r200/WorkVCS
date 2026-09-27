# ProjectRef Routed Journal Admission Candidate Evidence

Status: Implemented and validated on isolated fixtures; no live migration,
activation, installation, Store delivery, or commit
Date: 2026-09-26
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
Contract: [ProjectRef Control Plane v2](../architecture/projectref-control-plane-v2.md)
Acceptance: [Registry v2 Migration and Acceptance](../architecture/projectref-registry-v2-migration-and-acceptance.md)
Roadmap position: round 1 of 11 complete; ten mainline rounds remain

## Authorized boundary

This round connects one real top-level caller to the accepted journal-first
route and adds a separate, default-off journal-admission activation surface.
All registry migration, activation, journal, contention, failure, and recovery
mutations used disposable fixture directories, registries, journals, and
Stores. No source-tree binary or Skill was installed. The live registry was
not migrated, neither activation marker was installed, no live intent was
admitted, and no ProjectRef, live target Store object, Git commit, push, tag,
release, deployment, or global Hook was created.

WorkVCS Goal, Plan, Task, Decision, and Evidence objects used to govern this
round are ordinary records in the already-bound work-governance Store. They
are not ProjectRef registry, routing-marker, capture-journal, or target-Store
mutations.

## Actual routed caller

The source-tree `capture` command now accepts the same tool-neutral locator
input used by ProjectRef reads plus an explicit `--value-reason` supplied by
the external durable-value decision.

- With registry v1, legacy capture without `--value-reason` retains the
  existing bound-Store behavior. A value-qualified capture instead resolves a
  target-neutral owner context, admits one immutable
  `legacy_cognition_v1` intent under the registry-derived quiescence lock,
  rechecks the exact v1 source digest, reports migration required, and writes
  no Store object.
- With registry v2, value qualification is mandatory. The caller runs the
  accepted order of explicit ProjectRef, semantic Project, Git common
  directory, and CWD. It requires exact active read-routing and
  journal-admission markers, uses the closed home/sidecar journal alias,
  supplies the observed registry revision and digest to core, and rechecks
  registry plus activation while holding the shared lock.
- An unbound higher-ranked semantic owner remains primary. It is journaled as
  unbound; it never falls back to a mapped repository or CWD, and this round
  does not bootstrap a ProjectRef.
- Replay with the same idempotency key and payload reuses the immutable intent.
  A conflicting payload fails closed. Neither path processes journal events or
  delivers the semantic manifest into a target Store.

Migration apply now also holds the shared journal-quiescence lock, so a v1
admission and v1-to-v2 replacement cannot both cross their authority boundary
concurrently. A retained v1 intent remains visible to rollback readiness after
migration and blocks restoration exactly like every other admitted intent.

## Separate journal-admission activation

The strict marker is
`<control-plane-root>/journal-admission-activation-v1.json` with scope
`project_ref_v2_journal_admission`. It is bound to one registry ID, revision,
and canonical digest and is distinct from the read-routing marker.

The source-tree command supports read-only `--preview` and `--status`,
digest-locked `--apply`, and exact-digest `--disable`. Apply requires exact
read activation. Apply and disable acquire the registry lock before the shared
journal-quiescence lock. Disable therefore excludes new admission before
removing the marker. Absence is off; malformed, stale, wrong-scope, mismatched,
or symlinked markers fail closed. Rollback readiness enumerates both supported
marker aliases and treats every non-absent state as a blocker.

Four injected post-install failures return
`routing_activation_install_indeterminate`. Three injected post-removal
failures return `routing_activation_disable_indeterminate`. In both cases the
operator must first use read-only status. Exact observed active/absent state
then permits only the matching idempotent apply/disable recovery; no uncertain
marker is overwritten or inferred.

## Fixture evidence

The final source candidate includes these named end-to-end tests:

- `cli_v1_routed_capture_is_target_neutral_and_survives_migration` proves v1
  target-neutral admission, idempotent replay, Store stability, migration
  serialization, and post-migration rollback visibility;
- `cli_v2_journal_admission_is_default_off_exactly_activated_and_store_free`
  proves the read prerequisite, distinct marker scope, absent/malformed/stale/
  wrong-scope/symlink fail-closed states, exact apply/disable and replay,
  unbound semantic precedence, immutable intent reuse, post-disable rejection,
  and byte-stable registry/Store state; and
- `journal_admission_activation_faults_are_indeterminate_and_recover_by_status`
  proves all four post-install and three post-removal failure boundaries,
  status-first recovery, exact reuse, and temp cleanup.

The existing migration, rollback, activation, quiescence-race, orphan-lock,
and v2 durable-write rejection fixtures also pass, so the new caller does not
weaken their fail-closed boundaries.

## Validation evidence

Validation against the final candidate passed with the local LLVM toolchain:

- `cargo test --workspace --all-targets --quiet`, including all 249 CLI tests
  and every core unit/integration target;
- strict workspace/all-target Clippy with warnings denied;
- schema v0.1 validation;
- the full CLI smoke workflow with `smoke_result=passed`;
- operator recovery maturity with `PASS`, 52 core error codes, 53 guide
  entries, zero missing/extra coverage, and retryability matching the core
  rule;
- Rust formatting; and
- `git diff --check` at closeout.

The LLVM environment was used because the Apple compiler launcher on this host
is blocked by an unaccepted Xcode license. No license, system configuration,
dependency lock, or installed WorkVCS binary was changed.

## Live non-operation proof

The closeout probe confirmed that the configured registry remained v1 at
`/Users/example/.codex/workvcs/project-bindings.json`, with SHA-256
`c57ca8aa9b1f4a7d3ff0b1db9a35384b7e3776f809b7fb44737498a0ee323ed4`, size
`2862`, mode `0644`, and inode `107118324`. The retained Hernes Store at
`/Users/example/.codex/workvcs/stores/projects/<redacted-project-store>.sqlite`
retained SHA-256
`f3f2cdc3c8ed6f842ddf20ec9b81f89a440ccf6d72f354bcd691b350081cd644`, size
`712704`, mode `0644`, and inode `106869852`.

Both home and registry-sidecar read/journal marker aliases, both capture-journal
aliases, the canonical journal-quiescence lock, rollback backups/snapshots, and
registry temp patterns were absent.

## Next confirmation gate

Round 2 should add immutable journal events, rebuildable projections, explicit
recovery status, and idempotent post-intent ProjectRef/bootstrap convergence.
It must remain fixture-only and stop before primary target Store delivery,
installation, live migration, either live activation, commit, push, release,
deployment, or global Hook work.
