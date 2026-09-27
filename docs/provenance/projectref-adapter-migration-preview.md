# ProjectRef Adapter Input and Registry Migration Preview Evidence

Status: Repair-aware preview implemented and locally validated; no migration applied
Date: 2026-09-24
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)

This document freezes the evidence boundary at the end of the preview round.
Statements below that the apply command did not yet exist are historical for
that checkpoint. The later isolated-fixture apply candidate is documented in
[ProjectRef Registry Apply Candidate Evidence](projectref-registry-apply-candidate.md);
the live registry still has not been migrated.

## Authorized boundary

This slice implements two inspection and input boundaries only:

- a tool-neutral `ContextLocatorProvider` invocation that produces one
  deterministic `ResolutionContext` from bounded provider evidence plus
  separately verified Git/CWD inputs; and
- `workvcs project registry-migrate --preview`, which strictly projects a v1
  registry into a deterministic v2 migration plan while opening every target
  Store read-only; and
- an optional provider-neutral `--repair-manifest` that can replace one proven
  historical path owner with a namespaced semantic owner in the preview while
  retaining the old locator as retired and preserving the exact target.

It does not implement migration apply, mutate a registry or Store, activate
the v2 resolver for current commands, install a binary or Skill, or enable a
global Hook.

## Focused automated evidence

The core integration test
`projectref_adapter_migration_preview` covers deterministic tool-neutral
adapter collection, bounded/secret-safe context and explanations,
provider/source spoofing, non-semantic adapter output, explanation-digest
mismatch, strict v1 input, total validation facts, stable mapping order,
unknown identities, duplicate identities, shared targets, and exclusion of
generated IDs and timestamps from preview. It also covers strict/bounded repair
manifests, source/key/target binding, weak or non-semantic evidence rejection,
duplicate repair rejection, target preservation, active semantic identity, and
retired historical identity.

The CLI tests named `cli_project_registry_migration_preview_*` cover repeated
text/JSON preview, invalid Store reporting, shared-target coincidence without
alias/link creation, source/target drift rejection, repeated repair preview,
and byte/metadata/directory invariance for the registry, manifest, Store,
SQLite sidecars, lock, and control-plane sidecar path.

The focused migration test files now contain 8/8 core tests and 8/8 CLI tests,
including three repair-specific tests in each layer. They cover canonical
multi-row ordering, retired Git root context, repaired invalid-target rejection,
manifest drift, and zero-write success.

The final source tree also passed:

- `cargo test --workspace --all-targets --locked` (including 219 CLI tests);
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
- `scripts/validate-schema-v0.1.sh`;
- `cargo fmt --all -- --check`; and
- `git diff --check`.

The macOS build used the already-installed Homebrew LLVM toolchain because
the Apple compiler launcher on this host is blocked by an unaccepted Xcode
license. No license, system configuration, dependency lock, or installed
WorkVCS binary was changed.

## Baseline configured-registry preview

The source-tree binary was run repeatedly against the configured registry. The
repository-safe path representation is:

```text
/Users/example/.codex/workvcs/project-bindings.json
```

Both runs returned the same canonical preview. The retained result was:

| Field | Result |
| --- | --- |
| source registry version | `1` |
| target registry version | `2` |
| source digest | `blake3-256:acb09e62afae0949eadba491039a0f65c387018f25c97e05136ec42d4b989175` |
| preview digest | `blake3-256:1500b3537568ba117f1ca7d50386c9b3086b01400a083ad02cecea1f66c525f3` |
| one-to-one mappings | `6` |
| valid bindings | `5` |
| invalid bindings | `1` |
| target-coincidence groups | `1` |
| apply eligible | `false` |
| registry/journal/Store written | `false / false / false` |

The invalid binding is the redacted v1 CWD identity and root represented as
`/Users/example/Projects/Hernes`. That path no longer exists, so canonical-path
validation fails closed with `query_invalid`. A directory currently exists at
`/Users/example/Projects/HeXun/Hernes`, but preview deliberately does not infer that
it is the same logical project or rewrite the v1 identity. Repairing that
binding requires explicit semantic evidence; the other directory is not used
as a substitute.

The coincidence group contains the WorkVCS and work-governance Git-common-dir
bindings. They retain two one-to-one mapping rows and one shared-target
observation; preview creates no alias, ProjectRef, ProjectLink, or merge.

## Repair-aware Hernes preview

Current Codex application metadata independently identifies one redacted source
task as belonging to one redacted ChatGPT Project. The task's original
user-selected artifact path is represented as
`/Users/example/Projects/Hernes`; therefore the stale v1 CWD row is historical
execution/artifact context for that semantic Project, not evidence for a new
filesystem-owned project. The similarly named current directory under
`/Users/example/Projects/HeXun/Hernes` was not used as identity evidence.

The one-row repair manifest was bound to:

- source registry digest
  `blake3-256:acb09e62afae0949eadba491039a0f65c387018f25c97e05136ec42d4b989175`;
- redacted v1 key `cwd:/Users/example/Projects/Hernes`;
- unchanged target digest
  `blake3-256:2657ce87938c183ee627fce2a57f29124d7d6455e8199000dc8ce924732cb69c`;
- semantic locator provider `chatgpt`, kind `project_id`, with the normalized
  value redacted, under a stable local-installation namespace derived from the
  canonical Codex home; and
- bounded project-metadata evidence digest
  `blake3-256:497ce279e0c9388eb3b8690d9c0f9859c302b1be3ce0d0d10d5a20e552af9bd1`.

The exact validated manifest remains in the bound local WorkVCS state and is
not committed to the repository. This repository-safe shape deliberately
redacts the absolute user path, installation namespace, Project identifier,
and evidence digest. Round 8 must regenerate a fresh exact manifest from live
evidence and revalidate its digest rather than treating this display form as
apply input:

```json
{
  "schema_version": 1,
  "expected_source_digest": "blake3-256:acb09e62afae0949eadba491039a0f65c387018f25c97e05136ec42d4b989175",
  "repairs": [
    {
      "v1_binding_key": {
        "identity_kind": "cwd",
        "identity": "/Users/example/Projects/Hernes"
      },
      "expected_target_digest": "blake3-256:2657ce87938c183ee627fce2a57f29124d7d6455e8199000dc8ce924732cb69c",
      "semantic_locator": {
        "authority": "semantic_project",
        "provider": "chatgpt",
        "namespace": "codex-local:<redacted-digest>",
        "kind": "project_id",
        "normalized_value": "<redacted-project-id>",
        "assurance": "authoritative",
        "source_adapter": "codex-app-project-metadata/v1",
        "evidence_digest": "blake3-256:<redacted-digest>"
      },
      "historical_identity_disposition": "retire"
    }
  ]
}
```

The parser canonicalized the private local material before hashing;
presentation whitespace was not part of the manifest digest. Raw provider
payload, credentials, and private identifiers are not retained in Git.

Two consecutive source-tree previews produced the same result:

| Field | Result |
| --- | --- |
| preview version | `2` |
| repair manifest digest | `blake3-256:cb4a392116a56778113c5b4ceb63ce2830fd707c360915eb7c68756cfb01dd2f` |
| repair row digest | `blake3-256:4ede39c6a637222160f21726d2a713c7a2b16524781972bf2bb4b537d2bcd742` |
| preview digest | `blake3-256:4d03c5e7324580b36bf8f3c09ad112d3e60ab63510f0520f9769e16d2abcf7dc` |
| one-to-one mappings | `6` |
| valid / invalid bindings | `6 / 0` |
| ownership repairs | `1` |
| repaired active locator | `semantic_project`, `active`, `established` |
| historical v1 locator | `cwd`, `retired` |
| target-coincidence groups | `1` |
| apply eligible | `true` |
| registry/journal/Store written | `false / false / false` |

`apply_eligible=true` means only that the fully bound read-only candidate
passes current preview checks. It is not a migration, an activated route, or
authorization to add an apply command or replace the live registry.

## Zero-write observation

Before and after the retained repair-aware live preview, the probe compared
content hashes and size/mtime/ctime/mode/inode metadata for the entire current
WorkVCS control-plane tree (registry, five unique SQLite Stores, sidecars, and
content objects) plus the external repair manifest. It also compared the
presence/absence of the registry lock and control-plane sidecar. Every snapshot
was identical; both lock and sidecar remained absent. No migration backup,
replacement registry, journal, ProjectRef, or Store object was created.

After that observation window, ordinary WorkVCS lifecycle records for this
implementation Plan, its verification results, and closeout were intentionally
written to the already-bound work-governance Store. Those governance records
are outside the migration-preview zero-write claim; the live registry, Hernes
target, repair state, and routing state remained unchanged.

The baseline result proves that repair is never inferred; the repaired result
proves that the explicit manifest can produce a complete target-preserving
candidate without writes. Neither result proves that migration apply exists or
is authorized.

## Boundary challenge

A final adversarial self-review checked the failure boundaries that could turn
this preview into a second mutable authority or an implicit migration path:

- the default no-manifest preview retains its original version, fields, and
  configured-registry digest;
- the manifest is bound to the exact source, v1 key, target, semantic locator,
  and evidence, and duplicate or stale assertions fail before any Store waiver;
- repair waives only existence of the retired historical path, while canonical
  path syntax, external Store placement, target identity, content, and Store
  integrity remain mandatory;
- historical locators are retired rather than deleted, and cannot resolve an
  active owner;
- no `--apply` CLI variant or routing integration exists; and
- live preview left the complete configured control-plane tree byte- and
  metadata-identical.

No contradictory path, target-movement mechanism, implicit alias, or mutable
side authority was found. This is a self-challenge, not the separately deferred
independent review gate.

## Deferred gates

The following remain separate work and authority:

1. independently review the repair-aware contract and retained live evidence;
2. design and separately authorize migration apply, backup, fault handling,
   repair-manifest retention, and rollback probes;
3. review the still-current eligible real-registry preview and separately
   authorize an actual apply;
4. implement provider-specific adapter collection and activate the v2
   resolver only under a later gate;
5. complete journal recovery/delivery and consider any global Hook only at its
   own confirmation gate.
