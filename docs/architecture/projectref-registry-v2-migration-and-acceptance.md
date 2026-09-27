# ProjectRef Registry v2 Migration and Acceptance Contract

Status: Accepted design contract — roadmap round-5 source candidate and complete 93-row acceptance ledger present; live operations remain pending
Date: 2026-09-23
Last updated: 2026-09-27
Parent: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
Schema: [ProjectRef Control Plane v2 Contract](projectref-control-plane-v2.md)

## Purpose

This contract defines the only accepted v1-to-v2 registry migration and the
evidence required before the accepted ownership and capture-routing design can
be called implemented. It deliberately separates four future authority gates:

1. accept the semantic contract;
2. authorize implementation;
3. inspect a read-only migration preview against the real registry; and
4. separately authorize migration apply.

The user satisfied gate 1 on 2026-09-23 and then authorized bounded parts of
gate 2: the strict registry model, pure resolver, CaptureIntent validation,
atomic/idempotent intent admission, the generic adapter-input layer, the
read-only migration preview, the evidence-bound historical ownership-repair
preview, the digest-locked apply/recovery candidate, ordinary v1/v2 read
consumers, unified locator input, and the digest-bound v2 read-routing
activation candidate, followed by the digest-locked v2-to-v1 rollback
candidate, the shared journal-admission/rollback quiescence candidate, and the
actual routed capture plus separate journal-admission activation candidate,
immutable post-intent recovery events, deterministic projections, read-only
recovery status, explicit digest-locked first-write binding convergence, and
primary Store delivery with receipt recovery, followed by CaptureGroup
projection, immutable secondary-project reference delivery, missing-only
reference retry, completion receipts, and read-only recall by secondary
ProjectRef, followed by the first concrete CLI integration adapter, explicit
stronger-locator attachment, the full acceptance ledger, operator material,
and independent adversarial review. Mutating paths have been exercised only
on isolated fixtures.
Gate 3 was exercised
against the real configured registry on 2026-09-24 and remained zero-write.
The apply candidate, rollback state probe, actual rollback candidate, both
quiescence race orders, and orphan-lock recovery were then exercised only on
isolated fixtures. Read-routing activation apply was likewise exercised only
inside temporary fixtures. Gate 4 has not been exercised: the live registry
has not been replaced. Live read-routing activation, live journal admission,
live rollback, live semantic Store delivery,
live secondary-reference delivery, installation, commit, push, release, and a
global Hook remain outside the authorization. The complete feature and every
remaining migration gate remain subject to the acceptance evidence below.

## Migration invariants

The migration MUST satisfy all of the following:

1. **One v1 binding becomes one ProjectRef.** The mapping is never
   many-to-one, including when targets, paths, or basenames are equal.
2. **Target preservation is exact.** `store_path`, `store_id`, `workspace_id`,
   and `branch_id` retain their v1 values. Stores and their contents are not
   opened for mutation by registry migration.
3. **Identity preservation is explicit.** By default, the v1 identity
   kind/value becomes one active v2 identity locator on the new ProjectRef. The
   v1 root may become a context locator when it is not already represented by
   that identity. An explicit ownership-repair row instead makes one verified
   semantic locator active and retains the v1 identity and distinct root only
   as retired historical locators.
4. **No inferred aliases.** Equal targets produce an audit-only
   `possible_shared_target` observation. They do not produce a ProjectLink,
   locator reassignment, ProjectRef merge, or preferred owner.
5. **No invented history.** Migration creates no CaptureGroup, cross-project
   reference, Record, Knowledge, Evidence, Goal, Plan, Task, Session, Claim,
   ChangeSet, or WorkStateCommit.
6. **No implicit migration.** Discovery, recall, resume, audit, preview, and
   every other read-only operation perform zero migration writes.
7. **One mutable registry.** Apply atomically replaces the v1 registry with
   v2. There is no live v1 shadow registry, dual write, or fallback authority.
8. **Recoverability is bounded and testable.** The v1 bytes are retained in a
   digest-named backup. Rollback is allowed only before any v2-dependent
   mutation or capture intent exists.
9. **`--cwd` remains accepted.** It supplies a CWD and potential Git locator to
   the v2 resolver; it no longer asserts that CWD is the highest owner.
10. **Store validation stays strict.** Preview reports invalid bindings but
    writes nothing. A repaired row may waive only filesystem existence of the
    retired v1 identity/root path. It never waives canonical path syntax,
    external Store placement, or complete Store, Workspace, Branch, content,
    and integrity checks. Apply fails closed unless every selected binding
    passes its applicable complete checks.

## Current source-tree migration command contracts

The accepted command surface is shown here to make migration behavior
testable. Preview, digest-locked apply, read-only rollback state inspection,
and explicit digest-locked rollback exist in the current source tree. Both
mutation paths are candidates validated only on isolated fixtures; they are
not installed and have not been run against the live registry. A successful
preview or probe never authorizes mutation.

### Read-only preview

```text
workvcs project registry-migrate \
  --preview \
  [--registry PATH] \
  [--repair-manifest PATH] \
  [--format text|json]
```

Preview MUST:

- open the registry and every Store read-only;
- take no registry or journal write lock that creates a file;
- create no directories, backup, temp file, ProjectRef, Store object, or
  metadata;
- return both the source registry digest and preview digest;
- enumerate every one-to-one mapping in stable v1 binding-key order;
- report target-coincidence groups separately;
- report validation failures and set `apply_eligible=false`; and
- produce the same preview digest for the same canonical v1 bytes and
  validation facts.

Without `--repair-manifest`, preview version and canonical fields remain the
original v1-preview contract. With a valid manifest, preview version is `2`,
the manifest digest and each repair digest are emitted, and those fields are
covered by the preview digest. Supplying a manifest never mutates or activates
the repair.

The preview digest covers:

- source digest and source version;
- normalized ordered mapping rows;
- exact target tuples;
- planned locator classifications and maturity;
- target-coincidence groups;
- validation results; and
- target registry version.

For a repair-aware preview the digest additionally covers the repair-manifest
digest, repair count, semantic locator/evidence fields, retired historical
locator plan, and exact expected target digest.

Generated UUIDv7 values and timestamps are excluded so preview remains
deterministic. They are assigned only during apply and returned in its mapping
receipt.

### Ownership-repair manifest

The optional manifest is a generic provider-neutral input with schema version
`1`. It contains one or more rows of this form:

```json
{
  "schema_version": 1,
  "expected_source_digest": "blake3-256:<digest>",
  "repairs": [
    {
      "v1_binding_key": {
        "identity_kind": "cwd",
        "identity": "/historical/path"
      },
      "expected_target_digest": "blake3-256:<digest>",
      "semantic_locator": {
        "authority": "semantic_project",
        "provider": "provider-family",
        "namespace": "tenant-or-stable-local-installation",
        "kind": "project_id",
        "normalized_value": "provider-project-id",
        "assurance": "authoritative",
        "source_adapter": "adapter-name/v1",
        "evidence_digest": "blake3-256:<digest>"
      },
      "historical_identity_disposition": "retire"
    }
  ]
}
```

The parser rejects unknown fields, unsupported schema/disposition values,
empty or oversized manifests, unbounded/control-bearing locator scalars,
non-semantic authority, `observed` assurance, duplicate v1 keys, and one
semantic locator assigned to multiple v1 rows. Registry binding additionally
requires exact equality for the source digest, exactly one matching v1 row,
and the exact target digest. Repairs are sorted canonically before their digest
is computed.

The manifest asserts semantic ownership; it is not a path-rewrite map. It
cannot change `store_path`, Store ID, Workspace ID, Branch ID, or another
mapping row. The evidence digest binds the bounded external proof used by the
adapter/operator, while raw provider payload, credentials, and transcripts do
not enter the manifest. Provider-specific evidence collection stays outside
the core parser.

### Explicit apply

```text
workvcs project registry-migrate \
  --apply \
  --expected-source-digest DIGEST \
  --expected-preview-digest DIGEST \
  [--repair-manifest PATH] \
  [--registry PATH]
```

Apply MUST refuse to run unless:

- the source is version 1;
- the exact source and preview digests match current recomputation;
- the preview is apply-eligible;
- the registry lock is acquired;
- all binding and Store validations still pass under that lock; and
- no conflicting temp or backup artifact exists.

When the confirmed preview used ownership repair, apply MUST receive and
revalidate the exact same manifest; its digest must match the digest covered by
the expected preview. Omitting or substituting it changes the preview digest
and fails closed.

Apply MUST NOT accept a flag that skips preview-digest validation.

The current candidate implements this contract. It supports text output only,
re-reads the source, repair manifest, preview facts, and every Store under the
registry lock, and rechecks the exact v1 bytes immediately before rename. It
does not activate routing or mutate a Store.

### Read-only rollback state

```text
workvcs project registry-migrate \
  --rollback-check \
  --expected-installed-digest DIGEST \
  --expected-backup-digest DIGEST \
  [--registry PATH]
```

This command never restores a file. For installed v2 it validates the registry,
revision and migration receipt, exact raw backup bytes, canonical v1 source
digest, mapping coverage, all Store targets, absent activation, and empty
capture journals. It also verifies that registry and backup bytes stay stable
during the probe and reports `v2_ready` or `blocked`. After a rollback it
validates the exact current v1 bytes against the retained v1 backup and the
digest-named exact v2 snapshot, then reports `v1_restored` or `blocked`. A
successful state is evidence for an explicit separately authorized action; the
probe itself never writes.

`rollback_apply_safe=true` means the observed state is `v2_ready` for a
separately authorized restore. `rollback_reentry_safe=true` is narrower: it is
reported only for an already verified `v1_restored` state. Readiness to mutate
and safety of a no-write repeat are not interchangeable.

For a standard-name `project-bindings.json`, the probe fails closed across both
supported aliases regardless of whether the invocation selected the registry
through WorkVCS home or an explicit `--registry`: it checks the home-root and
registry-sidecar activation markers and both corresponding capture-journal
roots. Any present activation alias or unreadable/non-empty journal alias
blocks readiness. Canonically equivalent home paths are deduplicated.

The source candidate now closes the scan race with
`<canonical-registry-path>.journal-quiescence.lock`. Every supported journal
alias derives that same path. The registry-coupled journal API derives its root
from a closed standard-home/sidecar alias choice, rather than accepting an
arbitrary caller root. After acquiring the lock, core admission itself requires
the expected v2 revision and canonical digest; any caller check can only add
restrictions. Core repeats the exact identity read after that check. Thus a
no-op or registry-mutating callback cannot bypass the identity revalidation.
Admission revalidates after acquiring it;
rollback acquires it after the registry lock and holds it across the first
scan, snapshot/candidate work, atomic restore, and post-install verification.
Both race orders pass in isolated fixtures. V2 durable-write admission and
delivery nevertheless remain disabled because no live adapter route is wired
or authorized in this slice.

## One-to-one mapping rules

For each ordered v1 binding, apply creates:

1. one UUIDv7 ProjectRef;
2. one identity locator;
3. zero or one additional CWD context locator from `root`;
4. one binding containing the exact v1 target values; and
5. one mapping-receipt row from the v1 identity key to ProjectRef ID.

The migration receipt describes only ProjectRefs created by that migration.
After migration, an explicit first-write recovery may add a ProjectRef,
locator, and binding without inventing a migration mapping row. Receipt
validation therefore requires exact coverage of migration-created
ProjectRefs, not every ProjectRef that may later exist in the registry.

The v2 registry receives one new `registry_id`. Its initial `revision` is `1`.
Migrated ProjectRefs use `created_by=migration` and the apply timestamp.

V1 identity conversion is:

| v1 `identity_kind` | v2 authority | provider | kind | maturity |
| --- | --- | --- | --- | --- |
| `git-common-dir` | `repository` | `git` | `git_common_dir` | `established` |
| `cwd` | `cwd` | `filesystem` | `canonical_directory` | `provisional` |

The locator namespace is `registry:<registry_id>` because these path locators
are local to the migrated control plane and v1 did not retain a portable host
or tenant identity. Later adapter evidence may attach a properly namespaced
semantic locator without changing the ProjectRef or target.

Any unknown v1 identity kind fails preview validation. It is never coerced to
CWD.

For an accepted repair row, conversion instead creates one active semantic
identity locator with the manifest's provider/namespace/kind/value/assurance
and evidence digest. It also creates a retired locator for the v1 identity and,
when distinct, a retired CWD context locator for the v1 root. The ProjectRef is
`established`. The exact v1 target tuple is unchanged, and the retired locator
never participates in owner resolution.

If two or more binding rows have the same exact Store/Workspace/Branch target,
apply creates one `possible_shared_target` observation containing the distinct
ProjectRefs. The Store path participates in the target digest and remains
visible in the preview. No link is created.

## Atomic apply sequence

While holding the existing cross-process registry lock, apply performs this
ordered sequence:

1. reread and digest the current v1 registry;
2. reread and validate the exact repair manifest when the preview used one;
3. recompute and validate the preview digest;
4. read-only validate every Store target and binding;
5. serialize and validate the complete v2 candidate in memory;
6. write the exact v1 bytes to
   `<registry>.v1.<full-source-digest-hex>.bak`, sync the file and parent, and
   verify both its canonical v1 source digest and raw-byte backup digest;
7. write the v2 candidate to a unique same-directory temp file, sync it, read
   it back, and validate its schema and migration receipt;
8. atomically rename the temp file over the registry path;
9. sync the registry parent directory; and
10. reopen the installed v2 registry and verify its digest, mappings, and every
   Store binding before reporting success.

If a same-name backup already exists with the exact raw bytes and both digests,
apply reuses it. If its bytes differ, apply fails closed. It never overwrites
an unmatched backup. The receipt stores the canonical `source_digest` and the
raw-file `backup_digest` separately so a later probe can detect semantically
equivalent byte replacement.

Failure before step 8 leaves the v1 registry authoritative and removes only
the exact uninstalled temp file when safe. Failure at or after step 8 reports
an indeterminate-install diagnostic and directs the operator to inspect the
actual registry version and digest; it never guesses that rollback occurred.

The apply and rollback replacement candidates are enabled only on Unix-family
platforms, where the implementation relies on same-filesystem rename over an
existing destination. Other platforms fail before locking or writing until a
platform-specific replace primitive and fault matrix are accepted.

## Explicit rollback boundary

Rollback is a separately explicit recovery operation, not an automatic branch
inside apply. The source-tree candidate is:

```text
workvcs project registry-migrate \
  --rollback \
  --expected-installed-digest DIGEST \
  --expected-backup-digest DIGEST \
  [--registry PATH]
```

It may restore the verified v1 backup only when all of these are true:

- the installed registry still has `revision=1` and the matching migration
  receipt;
- no ProjectRef, locator, binding, link, or observation changed after apply;
- every supported capture-journal root is absent or has empty intents, events,
  projections, and locks directories;
- every home-root or registry-sidecar alias of the digest-bound read-routing
  activation marker is absent, so changing how the same registry was selected
  cannot bypass the gate; and
- the operator supplies the expected installed-v2 and backup digests.

Rollback acquires locks only in `registry -> journal quiescence` order. The
quiescence identity is derived from the canonical registry path, not a selected
journal root, so configured-home and sidecar aliases converge. A present
unowned/orphan lock blocks both rollback readiness and rollback apply. It is
never stolen by timeout, age, or PID inference; exact removal requires an
operator to prove that no owner remains. This candidate implements no live
lock-recovery command.

Before replacement, rollback preserves the exact canonical stored v2 bytes at
`<registry>.v2.<installed-digest>.rollback.bak`. It then writes the exact v1
backup bytes to a same-directory candidate, syncs and validates them, rechecks
every precondition under the registry lock, and atomically renames the
candidate over the registry. Failures before rename report
`registry_rollback_failed`; the v2 registry remains authoritative and an exact
snapshot may remain reusable. Failures after rename report
`registry_rollback_install_indeterminate`; recovery begins with
`--rollback-check`, never an automatic retry. A repeated explicit rollback is
a no-write success only when the current v1 bytes, v1 backup, v2 snapshot,
migration receipt, activation/journal state, and both digest locks all match.

Otherwise rollback fails closed. Forward repair or a separately designed
consolidation is required. Registry rollback never reverts Store Work-State.
The zero-use gate and shared lock serve different purposes: the scan proves no
prior v2 use, while the lock prevents a new admission from crossing that proof.
Neither activates future journal admission by itself.

The v1 backup is not a second live authority. Normal commands never discover
or write through it.

## Compatibility during the bounded transition

Preview plus the separately explicit apply, rollback, and rollback-state
mechanics are implemented. Ordinary read consumers distinguish v1 from v2. V1 discovery,
list, recall, resume, audit, closeout, and receipt reads retain their verified
path lookup and report `registry_version=1` plus
`migration_required=true`. Supplying an explicit ProjectRef or semantic
locator against a v1 read fails closed rather than silently ignoring the higher owner.
V2 list/status may inspect an inactive registry, while normal v2 read routes
require the exact digest-bound read-routing marker. Marker absence is
default-off; stale or malformed markers fail closed. Legacy v1 capture without
an explicit value reason retains the compatibility route. A value-qualified
v1 capture accepts tool-neutral semantic input and persists a target-neutral
intent without a Store write. Under v2, `capture` requires exact read-routing
and separate journal-admission markers, admits the intent, and stops before
ProjectRef bootstrap or Store delivery. Other cwd-based mutations continue to
reject v2. The migration and activation apply candidates MUST NOT be used
against live state without their later explicit gates.

A v2-capable binary encountering a v1 registry behaves as follows:

| Operation | v1 behavior |
| --- | --- |
| Read-only discovery, list, recall, resume, audit, or migration preview | May use a transient in-memory compatibility projection; reports `registry_version=1` and `migration_required=true`; performs no write. |
| Routed durable capture | Persists a target-neutral intent when the control-plane journal is writable, then reports `registry_migration_required`; performs no target Store write until v2 resolution is available. |
| Registry mutation or ProjectRef/locator/link mutation | Fails as `registry_migration_required`. |
| Explicit Store-path operations unrelated to registry routing | Retain their existing Store-local contract. |

Once v2 is installed, the registry has one writable format. An old v1-only
binary is expected to reject version 2; WorkVCS does not keep a v1 mirror for
it. The migration implementation and compatibility reader may be retired only
under a later explicit compatibility decision.

## Acceptance matrix

The accepted feature is not complete unless every required case below has an
automated test or a named, reproducible integration probe with retained
evidence.

### A. Ownership resolution

| ID | Required scenario | Expected result |
| --- | --- | --- |
| R-01 | Explicit valid ProjectRef plus semantic/Git/CWD evidence | Explicit ProjectRef is primary; other evidence is retained as context. |
| R-02 | Verified semantic Project plus mapped Git and CWD | Semantic ProjectRef is primary; repository is related context. |
| R-03 | Authoritative semantic ID equals verified mirror-derived ID | One semantic ProjectRef wins with both evidence sources visible. |
| R-04 | Authoritative semantic ID conflicts with mirror-derived ID | Authoritative ProjectRef wins; `context_mismatch` is explicit; no alias is created. |
| R-05 | No semantic adapter, mapped Git and CWD | Git ProjectRef is primary. |
| R-06 | No semantic or Git identity, mapped CWD | CWD ProjectRef is primary and provisional. |
| R-07 | Unclaimed semantic Project plus already-bound Git context | Semantic owner is `unbound`; resolver does not fall through to the repository. |
| R-08 | Durable write with one unclaimed semantic locator | Intent is durable first; one established semantic ProjectRef/Store binding is created idempotently. |
| R-09 | Durable write with unclaimed Git locator and no semantic owner | Intent is durable first; one established repository ProjectRef/Store binding is created idempotently. |
| R-10 | Durable write with only unclaimed CWD | Intent is durable first; one provisional ProjectRef/Store binding is created idempotently. |
| R-11 | Read-only request with unclaimed winning locator | `unbound`; registry, journal, Store, and filesystem remain byte-for-byte unchanged. |
| R-12 | Equal-rank distinct candidates | `ownership_conflict`; no target Store write. |
| R-13 | Attach stronger unclaimed locator to provisional ProjectRef | Same ProjectRef and target remain; maturity becomes established. |
| R-14 | Attach locator already claimed by another ProjectRef | `locator_already_claimed`; neither ProjectRef changes. |
| R-15 | Same display name or basename across projects | No alias, merge, or ranking effect. |
| R-16 | Semantic provider unavailable | Resolver degrades to verified Git, then CWD, without provider-specific failure in core. |
| R-17 | Explicit ProjectRef is missing or invalid | Operation fails closed; no semantic/Git/CWD fallback occurs. |

### B. Capture durability and recovery

| ID | Required scenario | Expected result |
| --- | --- | --- |
| C-01 | Explicit read-only or no-record decision | No intent, event, projection, registry write, or Store write. |
| C-02 | Journal path is unavailable before intent install | `capture_not_persisted`; no target Store write. |
| C-03 | Ownership unresolved after intent install | `pending_resolution`; complete semantic payload remains replayable. |
| C-04 | Binding or Store invalid after intent install | Pending state plus exact recovery action; no alternate target is guessed. |
| C-05 | Crash after target commit but before receipt event | Retry reuses target idempotency result and records the missing receipt without duplicate objects. |
| C-06 | Primary succeeds and secondary reference fails | Canonical record remains committed once; state is `pending_references`; retry creates only missing references. |
| C-07 | Same idempotency key, same payload, and same CaptureGroup (including `null`) | One intent/result is reused. |
| C-08 | Same idempotency key with a different payload or any CaptureGroup drift | `capture_idempotency_conflict`; neither intent is overwritten. |
| C-09 | Cross-project capture requests two canonical deliveries | Validation fails before intent admission. |
| C-10 | Canonical Record later changes | Existing secondary reference stays pinned; no silent follow-latest behavior. |
| C-11 | Secondary project needs an independent conclusion | New local Record has its own ID and explicit `derived_from` provenance. |
| C-12 | Secret-bearing or oversized manifest | Admission fails before journal persistence; diagnostic identifies policy class without echoing secret data. |
| C-13 | Legacy v1 manifest becomes stale while pending | Original intent remains intact and reports `legacy_manifest_upgrade_required`; no implicit rebase. |
| C-14 | Registry/journal/Store faults injected at every durable step | Recovery never reports an absent write as completed or duplicates a committed semantic object. |
| C-15 | Lower-ranked context has no ProjectRef and no requested delivery | Locator evidence remains recoverable; no secondary ProjectRef or Store is created merely to mirror context. |
| C-16 | Recall by a secondary ProjectRef with an immutable reference | Control-plane association is discoverable without creating a Store-local cross-Workspace relation. |
| C-17 | Cross-project intent is admitted before its semantic primary ProjectRef exists | Group retains the primary evidence digest; resolution adds exactly one canonical member without rewriting the intent. |
| C-18 | Recovery events are replayed or the projection is deleted, stale, or malformed | Immutable events remain authoritative and rebuild the same canonical projection bytes; exact replay adds no duplicate event. |
| C-19 | An unbound semantic, Git, or CWD owner is recovered explicitly | Exactly one ProjectRef, locator, and binding is created; semantic/Git ownership is established and CWD-only ownership is provisional. |
| C-20 | Recovery resolves to conflict or unresolved ownership | The status remains pending resolution; no fallback ProjectRef, binding, or Store is created. |
| C-21 | A fault is injected after Store bootstrap, registry staging/install, binding event, delivery start, target commit, receipt, or projection replacement | Status-first forward recovery converges without duplicate identity or semantic object; pre-delivery failures may leave only a pristine Store, while a committed delivery remains exactly one semantic commit. |
| C-22 | Registry or projection bytes change after the operator records expected digests | Apply fails before mutation and requires a fresh read-only status. |
| C-23 | An immutable event has a sequence gap, filename mismatch, payload drift, or broken digest chain | Recovery fails closed; projection repair never masks authority corruption. |
| C-24 | Registry metadata changes after a primary receipt | The receipt is retained only for the same resolved ProjectRef and becomes current only after exact Store/Workspace/Branch revalidation; a changed target returns to `pending_primary` without target mutation. |
| C-25 | A delivery transition is invalid or a CaptureGroup canonical local ID is not a Record | Validation fails before immutable event installation or target Store mutation. |

### C. Migration

| ID | Required scenario | Expected result |
| --- | --- | --- |
| M-01 | Preview a valid v1 registry twice | Identical source and preview digests; no filesystem or Store changes. |
| M-02 | Preview includes an invalid Store/Workspace/Branch | Failure is listed and `apply_eligible=false`; no write. |
| M-03 | Apply after source registry changes | Digest mismatch; no backup or replacement is installed. |
| M-04 | Apply valid v1 registry | One v2 ProjectRef per binding; exact targets preserved; installed registry validates. |
| M-05 | Two v1 bindings share one target | Two ProjectRefs plus one `possible_shared_target` observation; zero links or merges. |
| M-06 | Unknown v1 identity kind | Preview is ineligible; no coercion. |
| M-07 | Crash/fault before atomic rename | Original v1 registry remains authoritative and valid. |
| M-08 | Fault after rename | Diagnostic requires actual-version/digest inspection; no automatic rollback claim. |
| M-09 | Concurrent apply or registry mutation | Existing registry lock permits one winner; loser revalidates and exits safely. |
| M-10 | Safe rollback before any v2 use | Exact verified v1 bytes restored atomically. |
| M-11 | Rollback after v2 intent or registry mutation | Fails closed. |
| M-12 | Read-only command sees v1 | No implicit migration; explicit migration-required metadata is returned. |
| M-13 | Historical Stores contain related Records | No historical CaptureGroup or cross-project reference is invented. |
| M-14 | Preview one stale historical CWD with an exact semantic ownership-repair row | Semantic locator is active, old identity/root locators are retired, exact target is preserved, and no write occurs. |
| M-15 | Repair manifest source digest is stale | Preview fails closed before Store validation; no output is apply-eligible and no write occurs. |
| M-16 | Repair manifest target digest differs from the matched v1 row | Preview fails closed; no target substitution or write occurs. |
| M-17 | Repair is ambiguous, duplicated, weak, non-semantic, unknown-field, or oversized | Strict validation rejects the manifest without a registry, journal, or Store write. |
| M-18 | Repeat the same repair preview | Source, manifest, repair, target, and preview digests are identical. |
| M-19 | Omit the repair manifest after a repaired preview | Default preview remains unchanged and the historical missing path remains invalid; repair is never implicit. |
| M-20 | Repaired historical path is missing but target Store/Workspace/Branch/content is invalid | Target validation still fails and `apply_eligible=false`; historical repair cannot bypass Store integrity. |
| M-21 | Repaired `git-common-dir` row points at a Store below the historical repository parent | Preview remains ineligible even when the historical identity and linked-worktree root are missing. |
| M-22 | Explicit standard-name registry has an intent in the sibling WorkVCS-home journal | Rollback readiness is false; the fallback journal and sibling-home journal are both inspected. |
| M-23 | Activation fails after atomic marker installation | `routing_activation_install_indeterminate`; status observes the exact active marker and exact reapply is idempotent. |
| M-24 | Marker is malformed, stale, or symlinked | Status reports invalid or stale and ordinary reads plus apply remain fail-closed; no registry or Store write occurs. |
| M-25 | Registry v2 is selected by a cwd-based durable-write command | `capture` is journal-only and activation-gated; Plan admit/evolve, Receipt issue/consume, ensure, and bind reject before registry or Store mutation until their ProjectRef-v2 routes exist. |
| M-26 | WorkVCS home supplies the registry | The activation marker is located at the configured home root, not beside the registry, and remains digest-bound to the selected snapshot. |
| M-27 | Fault before rollback rename | V2 remains authoritative; only exact reusable snapshot state may persist and candidate temps are removed. |
| M-28 | Fault after rollback rename | `registry_rollback_install_indeterminate`; the probe observes exact `v1_restored` or `blocked` state before any retry. |
| M-29 | Repeat rollback after exact successful restore | No registry, backup, snapshot, journal, or Store write; the command reports verified reuse. |
| M-30 | Rollback sees activation, journal use, digest drift, a noncanonical/symlinked snapshot, or conflicting artifact | Fails closed before snapshot or registry replacement. |
| M-31 | The same standard registry is selected once through WorkVCS home and once through explicit `--registry` | Rollback enumerates both activation and journal aliases; any use through either alias blocks before snapshot or replacement. |
| M-32 | Durable journal admission races with rollback | Both derive one canonical-registry quiescence lock. Admission-first persists an intent and blocks rollback; rollback-first restores v1 while admission either times out fail-closed or fails its post-lock revalidation before persistence. |
| M-33 | Apply or rollback is requested on a non-Unix platform | Fails before lock or write until a platform-specific atomic-replace contract is implemented and fault-tested. |
| M-34 | A crash leaves the journal quiescence lock file | Readiness reports the present lock and both mutations fail closed; no age/PID-based steal occurs, and exact fixture recovery is required before retry. |
| M-35 | Value-qualified routed capture sees registry v1 | One target-neutral intent is persisted, migration-required is reported, and the target Store is byte-stable. |
| M-36 | Journal-admission marker is absent, malformed, stale, wrong-scope, mismatched, or symlinked | V2 capture fails before journal layout or Store write; the read-routing marker cannot satisfy the journal scope. |
| M-37 | Exact journal-admission apply and disable | Apply requires active read routing plus exact registry/candidate digests; disable requires the exact installed digest and excludes admission with the shared lock; exact repeats are idempotent, and every injected post-install or post-removal fault requires status-first recovery. |
| M-38 | Routed capture has an unbound semantic owner | The semantic locator remains primary, one immutable intent is admitted and replayed idempotently, and no ProjectRef or Store object is created. |
| M-39 | A post-migration first-write recovery adds a ProjectRef | The original migration receipt remains valid because its mappings still cover exactly the ProjectRefs created by migration; no false migration row is added. |

The
[round-5 acceptance ledger](../provenance/projectref-concrete-adapter-and-round5-acceptance.md)
accounts for all 93 R/C/M/N rows with a focused test, bounded probe, inspection,
or independent review. All executable rows except M-33 are proven for the
source candidate or isolated fixtures; this does not claim installation or
live execution. M-33 retains a fail-closed guard and remains explicitly
deferred until non-Unix atomic replacement and fault evidence exists. The
primary target-delivery path exists only behind the explicit recovery
candidate and has not been activated or run on the live control plane. M-32
is closed for the source candidate: the actual route uses the registry-coupled
constructor, supplies the observed v2 revision/digest, and rechecks separate
activation while holding the shared lock. It remains a live-release gate until
the same behavior is canaried after separately authorized activation. In
particular:

- `migration_candidate_preserves_targets_and_persists_one_to_one_receipts`
  and
  `repaired_candidate_activates_semantic_owner_and_retires_historical_path`
  cover candidate shape, exact targets, mappings, shared-target observation,
  and repaired locator states;
- `registry_migration_apply_pre_rename_faults_preserve_v1_and_clean_candidate_temp`
  injects five failures before replacement;
- `registry_migration_apply_post_rename_faults_are_indeterminate_and_never_auto_rollback`
  injects three failures after replacement;
- `registry_migration_apply_reuses_verified_backup_after_recoverable_failure`,
  `registry_migration_apply_rereads_exact_repair_manifest_under_lock`,
  `registry_migration_apply_rejects_invalid_target_before_backup`, and
  `registry_migration_apply_lock_contention_preserves_v1_without_backup`
  cover retry, manifest drift, target validation, and concurrency boundaries;
  and
- `registry_migration_rollback_probe_blocks_on_journal_or_backup_byte_drift_without_writes`
  proves that the probe rejects v2 use in either journal layout and
  exact-backup drift without restoring anything; and
- `registry_rollback_restores_exact_v1_and_reentry_is_verified_noop`,
  `registry_rollback_pre_rename_faults_preserve_v2_and_clean_candidate_temp`,
  `registry_rollback_reuses_verified_v2_snapshot_after_recoverable_failure`,
  `registry_rollback_post_rename_faults_are_indeterminate_and_probe_recovers_state`,
  `registry_rollback_blocks_activation_journal_and_digest_drift_before_snapshot`,
  and `registry_rollback_with_configured_home_blocks_sidecar_activation_alias`
  cover exact restoration, the five pre-rename and three post-rename fault
  boundaries, snapshot reuse, probe-first recovery, verified re-entry, and
  zero-use guards across both path-selection aliases;
- `journal_admission_first_persists_intent_and_forces_rollback_to_fail_closed`,
  `registry_rollback_first_restores_v1_and_admission_persists_nothing`,
  and
  `orphan_journal_quiescence_lock_blocks_rollback_until_exact_fixture_recovery`
  cover both winner orders, home/sidecar lock-identity convergence, post-lock
  admission revalidation, orphan reporting, and exact fixture recovery;
- `cli_project_registry_migration_repair_rejects_store_under_historical_git_repository_root`
  proves that a repaired Git identity cannot bypass the historical repository
  parent boundary;
- `routing_activation_post_install_faults_are_indeterminate_and_recover_by_status`
  injects every post-install boundary and proves status-first recovery plus
  exact idempotent reuse;
- `cli_v2_cwd_durable_write_matrix_fails_closed_without_registry_or_store_changes`
  covers every currently exposed cwd-based durable write and legacy registry
  mutation surface; and
- `cli_v2_routing_activation_uses_configured_home_marker_path` (including the
  inverse explicit-registry rollback probe) plus
  `cli_v2_routing_activation_rejects_malformed_stale_and_symlink_markers`
  cover both read-marker layouts and the invalid/stale marker states; and
- `cli_v1_routed_capture_is_target_neutral_and_survives_migration` plus
  `cli_v2_journal_admission_is_default_off_exactly_activated_and_store_free`
  cover target-neutral pre-migration admission, migration quiescence, separate
  marker scope, absent/malformed/stale/wrong-scope/symlink rejection, exact
  apply/disable, unbound semantic precedence, idempotent replay, rollback
  visibility, and zero target-Store writes.
- `journal_admission_activation_faults_are_indeterminate_and_recover_by_status`
  injects all four post-install and three post-removal boundaries, proves the
  dedicated indeterminate codes, observes exact active/absent state through
  read-only status, removes candidate temps, and permits only exact idempotent
  recovery; and
- `projection_rebuild_is_byte_equivalent_and_replay_has_no_duplicates`,
  `event_chain_gaps_reordering_and_payload_tampering_fail_closed`,
  `first_write_binding_converges_once_and_rejects_target_substitution`, and
  `repository_and_cwd_bootstrap_preserve_maturity_provenance` prove immutable
  authority replay, deterministic projection repair, fail-closed event
  corruption, and one-time ProjectRef/binding convergence; and
- `cli_capture_recovery_delivers_primary_once_and_reuses_receipt`,
  `capture_recovery_faults_converge_forward_without_duplicate_identity`,
  `stale_legacy_manifest_requires_upgrade_without_target_store_write`, and
  `capture_recovery_conflict_records_status_without_bootstrap_or_fallback`
  prove the explicit digest locks, read-only status, nine recovery fault
  boundaries, commit-before-receipt reuse, byte-stable legacy intent, no-write
  stale-legacy handling, forward convergence, and conflict/unresolved
  no-fallback behavior; and
- `commit_before_receipt_recovery_reuses_one_primary_result`,
  `primary_receipt_exposes_canonical_record_and_waits_for_references`, and
  `stale_legacy_manifest_is_detected_without_rewriting_the_intent`,
  `registry_revalidation_preserves_receipt_only_for_the_exact_target`,
  `invalid_delivery_transition_is_rejected_before_event_install`, and
  `capture_group_canonical_record_is_preflighted_before_target_write` prove
  target-neutral manifest materialization, complete primary receipt,
  deterministic projection rebuild, exact-target receipt retention, event
  prevalidation, one target commit, and the pending-reference handoff boundary
  in core; and
- `pending_references_retry_only_missing_and_secondary_recall_is_authoritative`,
  `invalid_secondary_transition_fails_before_event_install`, and
  `unresolved_group_adds_one_canonical_member_from_matching_locator_evidence`,
  plus the two CLI CaptureGroup recovery/recall tests, prove exact canonical
  version pinning, missing-only retry, fail-closed transition validation,
  immutable-authority projection rebuild, one-time unbound-primary
  resolution, Store-free recall by secondary ProjectRef, and completion-receipt
  recovery across the four round-4 fault windows.

M-10 is implemented and proven only in isolated fixtures. M-11 is proven for
the currently observable activation, journal, bootstrap, and primary-delivery
surfaces; the live route remains disabled rather than exercised. Focused
fixture tests now prove ordinary v1/v2 reads, default-off v2 routing,
exact-digest read and journal activation, status-first recovery after
post-install and post-disable faults, fail-closed semantic-owner precedence,
journal-only routed capture with zero target-Store writes, separately explicit
primary delivery and receipt recovery on fixture Stores, and control-plane-only
secondary association plus recall without a secondary Store open. This
evidence does not claim live migration, live activation, ProjectRef bootstrap
against the real registry, live semantic target delivery, live
secondary-reference delivery, or installation. It establishes the source
candidate and isolated-fixture contract for ProjectRef bootstrap, recovery
status, primary delivery receipts, and CaptureGroup reference convergence.

### D. Boundaries and non-regression

| ID | Required scenario | Expected result |
| --- | --- | --- |
| N-01 | Existing `--cwd` caller under v2 | CLI remains accepted and feeds semantic/Git/CWD resolver precedence. |
| N-02 | Linked Git worktrees | Git common-directory locator does not duplicate the repository ProjectRef. |
| N-03 | Two semantic Projects use the same repository | Distinct semantic ProjectRefs remain primary in their contexts; repository may be context for both. |
| N-04 | Store integrity failure | No registry or journal metadata is used to bypass complete Store validation. |
| N-05 | Cross-project CaptureGroup | No cross-Workspace Goal/Plan/Task/Record relation is created by routing metadata. |
| N-06 | Adapter emits raw provider payload or missing namespace | Evidence is rejected or redacted before persistence. |
| N-07 | Full current test suite | Existing Store, capture, recall, Plan, evidence, bundle, and integrity contracts remain green. |
| N-08 | Global Hook inspection | No Hook is installed or enabled by this implementation. |
| N-09 | Configuration/documentation inspection | Any changed locator or config surface is reflected in config examples, operator docs, and focused tests in the same implementation change. |
| N-10 | Independent adversarial review | Reviewer finds no second mutable authority, false cross-Store atomicity claim, implicit migration, or silent-loss path. |
| N-11 | Skill/integration sees valuable content and an unbound owner | It submits a journal-backed capture; binding absence is not converted to no-record. |
| N-12 | User-facing durability claim | It states “no silent loss after admission” and does not claim complete cognition capture while the global Hook is deferred. |

## Evidence required for future completion claims

An implementation completion report MUST include:

- the accepted ADR and promoted invariant identifiers;
- exact registry and journal schema validation results;
- focused test names for every matrix row;
- fault-injection coverage for journal, registry, Store, and receipt boundaries;
- a read-only preview from the real configured registry, with paths and IDs
  redacted only where necessary but digests retained;
- proof that preview changed no registry, Store, or journal bytes;
- for any ownership repair, the retained manifest/evidence digest and proof
  that the original target tuple was unchanged;
- an independent review result; and
- explicit disclosure of any deferred matrix row.

Passing unit tests alone is not evidence that the user's live registry was
migrated. A successful preview is not authority to apply migration. Running
the source-tree migration, rollback, or activation candidate against live
state, installation, durable-write routing, commit, push, release, and global
Hook activation remain separate actions.

## Accepted semantic checklist

ADR-0513 confirms exactly these choices:

1. ProjectRef is the stable project identity; locators and Store targets are
   separate.
2. Ownership order is explicit ProjectRef, semantic Project, Git, CWD, then
   pending; an unbound higher-ranked owner blocks fallback and lower-ranked
   contexts remain related.
3. Adapters are tool-neutral and namespaced; ChatGPT/Codex is one example.
4. Valuable capture is journal-first; read-only/no-record creates nothing.
5. One canonical mutable Record exists in the primary project; secondary
   material is immutable reference or explicitly derived local state.
6. Cross-Store completion is idempotent convergence, not one transaction.
7. V1 migration is preview-gated, one-to-one, target-preserving, explicit, and
   recoverable; evidence-bound ownership repair retires rather than deletes an
   incorrect historical locator, and shared targets do not prove shared
   identity.
8. The initial implementation excludes a global per-turn Hook and automatic
   historical CaptureGroups.

Any change to one of these eight items requires a later accepted decision; an
implementation may not silently revise this contract.
