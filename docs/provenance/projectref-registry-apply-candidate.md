# ProjectRef Registry Apply Candidate Evidence

Status: Implemented and fault-tested on isolated fixtures; no live apply, routing activation, rollback, installation, or commit
Date: 2026-09-24
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
Contract: [ProjectRef Registry v2 Migration and Acceptance Contract](../architecture/projectref-registry-v2-migration-and-acceptance.md)

## Authorized boundary

This slice adds a source-tree migration candidate with three explicit actions:

```text
workvcs project registry-migrate --preview ...
workvcs project registry-migrate --apply --expected-source-digest DIGEST --expected-preview-digest DIGEST ...
workvcs project registry-migrate --rollback-check --expected-installed-digest DIGEST --expected-backup-digest DIGEST ...
```

Only isolated temporary fixtures were passed to `--apply`. The configured live
registry was read only. The source-tree binary was not installed, ordinary
registry consumers were not switched to v2, no journal/routing path was
activated, no backup was restored, and no Git commit was created.

## Candidate behavior

Apply acquires the existing cross-process registry lock and then re-reads and
validates the exact v1 source, optional ownership-repair manifest, preview, and
every Store target. Both the expected source and preview digest are mandatory;
there is no skip flag. The exact v1 bytes are rechecked immediately before
rename.

The candidate builds and validates the complete v2 document in memory. Each
ordered v1 row receives one ProjectRef, exact target binding, planned locators,
and one durable v1-key-to-ProjectRef mapping receipt. Shared targets remain an
audit-only observation. A repaired row receives its active semantic locator
and retired historical locators without rewriting its Store target.

Backup and source digests intentionally cover different facts:

- `source_digest` hashes canonical v1 registry meaning; and
- `backup_digest` hashes the exact raw v1 file bytes.

The backup name includes the full canonical source-digest hex. A new backup is
written and synced through a same-directory temp file, atomically hard-linked
without overwrite, parent-synced, reread, and verified. An existing exact
backup is reusable; a byte mismatch, symlink, other digest-named backup, or
conflicting temp fails closed.

The v2 candidate is written to a same-directory create-new temp, receives the
source permissions, is synced and reparsed, then atomically renamed over the
registry. Parent-directory sync and complete installed-registry, receipt,
backup, and Store verification precede success.

Failure before rename reports `registry_migration_apply_failed`, preserves v1,
and removes only the exact temp files created by that attempt. Failure after
rename reports `registry_migration_install_indeterminate`; it never claims or
performs automatic rollback.

`--rollback-check` is read-only. It verifies the expected installed and raw
backup digests, revision-1 receipt and mapping coverage, canonical v1 backup,
all Store targets, stable registry/backup bytes during inspection, and that
the capture journal has no intents, events, projections, or locks. It only
reports `rollback_ready`; actual restoration is deliberately absent.
For an explicit standard-name registry it conservatively inspects both the
direct-registry fallback journal root and the sibling WorkVCS-home journal
root.

## Fault-injection evidence

The isolated harness injected failures at these durable boundaries:

| Boundary | Observed invariant |
| --- | --- |
| after backup temp write | v1 remained exact; no final backup or temp remained |
| after verified backup installation | v1 remained exact; exact backup remained reusable |
| after candidate temp write | v1 and backup remained exact; candidate temp was removed |
| after candidate temp sync | v1 and backup remained exact; candidate temp was removed |
| immediately before registry rename | v1 and backup remained exact; candidate temp was removed |
| immediately after registry rename | v2 remained installed; result was indeterminate; no rollback occurred |
| after parent-directory sync | v2 remained installed; result was indeterminate; no rollback occurred |
| before installed-registry verification | v2 remained installed; result was indeterminate; no rollback occurred |

Additional probes rejected stale source/preview digests, a repair manifest
changed after preview, an invalid Store target, held registry lock, mismatched
exact backup, other digest-named backup, and stale temp artifact. A retry after
the injected post-backup failure reused the exact backup and completed. The
readiness probe returned false for intents in both supported journal layouts
and for a byte-drifted backup, and never restored either registry version. A
repaired historical `git-common-dir` row was also rejected when its Store sat
below the historical repository parent, even though the identity and linked
worktree paths themselves were absent.

Validation passed with the repository's local LLVM toolchain:

- core candidate test target: 4 passed;
- CLI `registry_migration` filter: 18 passed;
- strict workspace clippy with warnings denied: passed;
- full workspace/all-target test suite: passed, including 229 CLI tests and
  every core integration target;
- schema v0.1 validation: passed; and
- full CLI smoke workflow: `smoke_result=passed`.

Independent review first found two material gaps: repaired Git rows did not
check the identity-parent repository boundary, and an explicit standard-name
registry could miss a sibling WorkVCS-home journal. Both were corrected with
dedicated regression cases. Targeted follow-up review found no remaining high
or medium risk in those corrections. The reviewer made no file changes.

## Live-state non-operation baseline

Before isolated apply testing, the configured live registry was still v1 at
`/Users/example/.codex/workvcs/project-bindings.json`, with SHA-256
`c57ca8aa9b1f4a7d3ff0b1db9a35384b7e3776f809b7fb44737498a0ee323ed4`,
size `2862`, mode `0644`, and inode `107118324`. The retained Hernes target
Store had SHA-256
`f3f2cdc3c8ed6f842ddf20ec9b81f89a440ccf6d72f354bcd691b350081cd644`,
size `712704`, mode `0644`, and inode `106869852`. No live migration backup,
registry temp, or registry lock existed.

The final closeout probe ran only `--preview` against the live registry. It
again reported registry v1, six mappings, five valid bindings, the one known
missing historical Hernes path, `apply_eligible=false`, source digest
`blake3-256:acb09e62afae0949eadba491039a0f65c387018f25c97e05136ec42d4b989175`,
and preview digest
`blake3-256:1500b3537568ba117f1ca7d50386c9b3086b01400a083ad02cecea1f66c525f3`.
Before and after that preview, both SHA-256 values and every recorded
size/mtime/ctime/mode/inode field above were identical. The registry remained
v1 and no live migration backup, temp, or lock appeared. WorkVCS records
written to the already-bound work-governance Store for this implementation
lifecycle are separate authorized governance writes and are not migration
writes.

## Remaining gate

This evidence did not by itself make the live registry v2-ready for normal use.
The next candidate—ordinary v1/v2 reads plus controlled read-routing
activation—was subsequently implemented and remains fixture-only; see
[ProjectRef v2 Read Routing and Activation Candidate Evidence](projectref-v2-read-routing-activation-candidate.md).
Only after a separate live migration authorization may the exact live preview
digests be supplied to `--apply`. Actual rollback remains a later separately
designed and authorized operation.
