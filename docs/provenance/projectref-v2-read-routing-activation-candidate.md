# ProjectRef v2 Read Routing and Activation Candidate Evidence

Status: Implemented and validated on isolated fixtures; no live migration, activation, installation, or commit
Date: 2026-09-24
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
Contract: [ProjectRef Registry v2 Migration and Acceptance Contract](../architecture/projectref-registry-v2-migration-and-acceptance.md)

## Authorized boundary

This slice completes the bounded source-tree candidate that precedes any live
registry cutover:

- ordinary project discovery, list, recall, cwd-based resume, and cwd-based
  currentness audit distinguish registry v1 and v2;
- a strict tool-neutral locator envelope feeds the same core resolver as
  provider invocations, without embedding ChatGPT, Codex, or another provider
  in core policy;
- ProjectRef-v2 ordinary reads are default-off and require an exact activation
  marker bound to the selected registry ID, revision, and digest; and
- the source tree can inspect, preview, or install that marker only through a
  separately digest-locked command.

All activation writes in this round targeted temporary isolated fixtures. The
configured registry was not migrated or activated. The source-tree binary and
Skill were not installed, durable write routing and journal delivery were not
enabled, actual rollback was not added, and no Git commit was created.

## Versioned read behavior

Registry v1 remains a supported read source. Its discovery, list, recall,
resume, and currentness-audit outputs identify `registry_version=1`, report
`migration_required=true`, retain verified Git-common-directory/CWD lookup,
and open the target Store read-only. Explicit `--project-ref` or
`--locator-context` input against v1 fails closed instead of silently ignoring
a higher-ranked owner.

Registry v2 list and activation status can inspect an inactive snapshot.
Ordinary reads require an active exact marker and then resolve in this order:

1. explicit ProjectRef;
2. verified semantic Project evidence;
3. verified Git common directory;
4. verified CWD; and
5. unresolved/pending.

A known but unbound higher-ranked locator blocks lower-ranked fallback.
Conflicting equal-rank evidence and an unknown explicit ProjectRef also fail
closed. The strict `--locator-context FILE` envelope contains only canonical,
bounded, already-verified semantic `LocatorEvidence`. Git and CWD remain
dedicated locally verified path channels.

The following cwd-based commands use this route:

```text
workvcs project discover --cwd PATH [--project-ref ID] [--locator-context FILE]
workvcs recall --cwd PATH [--project-ref ID] [--locator-context FILE]
workvcs resume --cwd PATH [--project-ref ID] [--locator-context FILE]
workvcs record currentness-audit --cwd PATH [--project-ref ID] [--locator-context FILE]
```

`project list` reads and verifies every v1 or v2 binding without selecting an
owner. Existing cwd-based closeout and receipt inspection use the same
versioned read-only discovery path when path evidence is sufficient.

Legacy direct registry mutation and durable write commands reject registry v2
with a journal-backed-route-disabled diagnostic. The read activation marker
cannot enable those commands.

## Activation contract

The command surface is:

```text
workvcs project routing-activation --status [--registry PATH]
workvcs project routing-activation --preview [--registry PATH]
workvcs project routing-activation --apply --expected-registry-digest DIGEST --expected-candidate-digest DIGEST [--registry PATH]
```

The strict marker contains only:

- activation format version `1`;
- scope `project_ref_v2_read_routing`;
- registry ID;
- registry revision; and
- canonical registry digest.

With a configured WorkVCS home the marker is
`<home>/routing-activation-v1.json`. A registry-only locator uses
`<canonical-registry-path>.d/routing-activation-v1.json`. Absence means off.
Malformed, symlinked, stale, or snapshot-mismatched markers are never accepted
as active.

Apply acquires the registry lock, re-reads registry v2, verifies every Store
binding read-only, checks both expected digests, refuses to replace any
non-absent nonmatching marker, writes a create-new same-directory temp file,
syncs and reparses it, atomically installs it without overwrite, syncs the
parent directory, and verifies the installed marker. Reapplying the exact
candidate reuses the existing marker. The result explicitly reports
`durable_write_routing_activated=false`, `registry_written=false`,
`journal_written=false`, and `store_written=false`.

Once the create-new hard link has installed the marker, any later temp cleanup,
directory-sync, injected, or verification failure is reported as
`routing_activation_install_indeterminate`. The operator must run read-only
`--status` against the same registry before retrying. The implementation never
deletes or overwrites the installed marker as recovery; an observed exact
active marker makes exact reapply an idempotent reuse.

The registry rollback-readiness probe now treats every non-absent activation
state as a rollback blocker. This prevents a v1 backup from becoming active
while a v2 read marker remains present.

## Fixture and validation evidence

The isolated CLI fixture proves:

- v1 ordinary reads report migration required and preserve registry and Store
  bytes and filesystem metadata;
- an installed v2 registry remains inspectable while ordinary reads are off;
- a wrong activation-candidate digest creates no marker;
- an exact candidate activates only fixture reads;
- all four post-install fault boundaries report the dedicated indeterminate
  code, remain visible as exact active state, leave no temp artifact, and
  permit exact idempotent reuse after status inspection;
- configured-home selection places the marker at
  `<home>/routing-activation-v1.json`, while malformed, stale, and symlinked
  marker fixtures remain fail-closed;
- discovery, recall, resume, and currentness audit then report v2 resolution;
- an unbound semantic owner blocks a mapped CWD, while an explicit valid
  ProjectRef wins and retains the other evidence as context;
- cwd-based capture, Plan admit/evolve, Receipt issue/consume, ensure, and bind
  all reject registry v2 before registry or Store mutation; and
- rollback readiness becomes false while the marker is active.

The core tests prove that direct already-verified adapter evidence produces the
same ordered, canonical unified locator input and that the activation marker is
strictly bound to one registry snapshot.

Validation passed with the repository's local LLVM toolchain:

- full workspace/all-target test suite, including 236 CLI tests: passed;
- focused core ProjectRef adapter/migration target: 9 passed;
- focused core registry/apply/activation target: 5 passed;
- strict workspace clippy with warnings denied: passed;
- independent follow-up review: no Blocker, High, or Medium findings; its two
  Low test-evidence gaps (the activation-temp filename predicate and direct
  stale/symlink read/apply rejection assertions) were corrected, and both
  focused tests plus the complete 236-test CLI target passed again;
- Rust formatting check: passed; and
- `git diff --check`: passed.

The local LLVM toolchain was used because the Apple compiler launcher on this
host is blocked by an unaccepted Xcode license. No license, system
configuration, dependency lock, or installed WorkVCS binary was changed.

## Live-state non-operation boundary

The closeout probe confirmed that the configured registry remained version 1
with six bindings. Its SHA-256 remained
`c57ca8aa9b1f4a7d3ff0b1db9a35384b7e3776f809b7fb44737498a0ee323ed4`,
size `2862`, mode `0644`, and inode `107118324`. The retained Hernes Store at
`/Users/example/.codex/workvcs/stores/projects/<redacted-project-store>.sqlite`
retained SHA-256
`f3f2cdc3c8ed6f842ddf20ec9b81f89a440ccf6d72f354bcd691b350081cd644`,
size `712704`, mode `0644`, and inode `106869852`.

No live migration backup, registry temp, registry lock, control-plane sidecar,
or routing activation marker existed. Both the configured-home marker and the
direct-registry sidecar marker were absent. WorkVCS lifecycle records written
to the already-bound work-governance Store are authorized governance records
and are not migration or activation writes.

## Remaining confirmation gate

This evidence does not authorize a live migration or activation. The next
bounded work should prepare the actual rollback command and its fault/recovery
tests, still on isolated fixtures. Only after that recovery path and a refreshed
eligible real-registry preview are independently accepted should a separate
gate consider live migration apply. Live read activation must remain a distinct
post-migration operation bound to the exact installed v2 digest. Installation,
durable write routing, provider-specific collection, and global hooks remain
separate decisions.
