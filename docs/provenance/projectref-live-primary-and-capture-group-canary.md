# ProjectRef Live Primary Delivery and CaptureGroup Canary Evidence

Status: Completed local-runtime evidence for roadmap rounds 11A, 11B, and 11C
Date: 2026-09-27
Parent: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)

## Scope and authority

This evidence closes the initial ProjectRef delivery roadmap on the configured
local WorkVCS control plane. The authorized scope was continuous execution of
11A through 11C: expose target-neutral `cognition_v2` and CaptureGroup input at
the public `capture` entry, install the exact reviewed local revision, and run
one bounded live primary-delivery plus cross-project-reference canary. It did
not authorize push, tag, public release, remote deployment, global Hook
installation, historical CaptureGroup backfill, rollback, or deletion of the
earlier legacy intent.

## Reviewed and canary-installed revision

The public route was implemented in local commit
`050ce23140f0449f527fd0fd92d74452cd099485`. Canary preparation then exposed a
real operator gap: `capture_group_id` was required, but the typed ID surface
could not generate one. No borrowed ID kind was used. Local commit
`615f946c11ea21152616dfccffe13c612a8d8059` added
`workvcs id new|validate --kind capture-group` and is the exact installed
revision.

The final release package was built from a clean checkout:

- package directory:
  `target/package/workvcs-aarch64-apple-darwin-615f946c11ea-20260927T142003Z-78849`;
- manifest `source_git_dirty=false` and exact full source commit as above;
- installed binary: `$HOME/.local/bin/workvcs`;
- installed binary SHA-256:
  `f6bad0f0c7903519d6a14bd204cfb1a75e3494cb89d1fa7a8226f04498ddf6f1`;
- installed Skill entry SHA-256:
  `5ecb3860eb48fab5625e8ca2f806f9e2cd1e3f1227a0900bc6671476d34b1377`;
- installed Skill tree-manifest SHA-256:
  `d978aa4ed59008373f7d82b0a96945956660426e203c4fa0c4170ac2ac3fe1d6`;
- packaged binary and Skill were verified after atomic installation; and
- source and installed Skill trees were byte-identical.

The public-route commit passed the full workspace test run, including all 261
CLI unit tests and all core integration suites, plus strict workspace clippy,
schema validation, and the complete 93-row ProjectRef acceptance-matrix
validator. After the typed-ID delta, its focused typed-ID test and the public
CaptureGroup end-to-end test passed again, followed by strict clippy and both
validators, before the final live write.

## Post-canary closeout correction

Plan closeout exposed one independent public-CLI omission: the Task core
correctly requires a non-empty structured rationale when entering
`cancelled`, but `workvcs task transition` did not expose a rationale input.
Marking the two replaced legacy Tasks `done` would have falsified their
outcome, so commit `4c52c040326077c651e7a6640a3ff3fa665e5f28` added
`--rationale-json`, passed the full workspace suite, strict clippy, schema
validation, and the 93-row ProjectRef matrix, and was installed before Plan
reconciliation.

That later package is
`target/package/workvcs-aarch64-apple-darwin-4c52c0403260-20260927T232419Z-78511`;
its packaged and installed binary SHA-256 is
`374080a8b9f858bdc893df5cfc643eee7917cec050fd6a605d56c5ac1b82f74d`.
The Skill content did not change: its entry and tree-manifest SHA-256 values
remain the values recorded above. This corrective install does not alter the
fact that the live CaptureGroup admission, delivery, and replay below were
executed with the exact clean `615f946c11ea...` canary revision.

## Live preconditions

The selected registry was
`$WORKVCS_HOME/project-bindings.json`, registry ID
`01a0e2e6-f2e6-77a2-b858-7631e40f1ad5`, revision `1`, semantic digest
`blake3-256:70dd5827fd065224b26368c37ca4ea112e5da1098f80361bb021e573bf6c4b2d`.
Both exact markers were active and matched that snapshot:

- read routing:
  `blake3-256:571b08a6b971fc929abf41eb519de1ff25512ff04bc6bc3702525bf2049a1150`;
- journal admission:
  `blake3-256:d0e1ea4ca9650fe7987e20ba3389db68034cccbd9153ce80dd7be3efe53f3567`.

The primary owner was the existing work-governance ProjectRef
`01a0e2e6-f2e6-77a2-b858-7690808a81a7`. The only secondary member was Hernes
ProjectRef `01a0e2e6-f2e6-77a2-b858-765e9d6a0cb3`. Because both identities and
bindings already existed, the canary required no registry revision change,
ProjectRef bootstrap, or activation refresh.

The prior live Capture `01a0e306-47aa-7372-9e44-62b8b4320803` remained an
immutable `legacy_cognition_v1` intent at
`legacy_manifest_upgrade_required`. Its intent SHA-256 was
`39c6edd571708b3e1a836c2c2aad80ed21d026a8d589ed828722a51ac2a60770` and its
stored projection SHA-256 was
`7ffdeb47ef5d576eec6b8cb198a7dd949509102825f60b52b0c2c1c67429fd02`
before the replacement canary.

## Admission and delivery result

The installed typed-ID command generated CaptureGroup
`01a0e33d-3e6c-7201-87f5-c9897c82af7a`. Public `capture` admitted new Capture
`01a0e33e-31e1-7b91-a698-4e06525377fd` with:

- `payload_kind=cognition_v2`;
- payload digest
  `blake3-256:98c32ad84552ca1f613581f38bd1440f5d1942e9bcdb0abf13da3f35dde95816`;
- explicit primary ProjectRef resolution;
- two CaptureGroup members; and
- `registry_written=false`, `project_ref_created=false`, and
  `store_written=false` at admission.

An exact admission replay returned the same Capture ID with
`admission_outcome=reused` and `journal_written=false`. The immutable new
intent SHA-256 is
`b24b3aee7b933ca8b1003c5b7ae617a0b5fd777fb39fbce0c5fd16de938396f7`.

Status then reported `pending_project` with no stored projection and supplied
fresh registry and projection digests. One explicit digest-locked recovery
apply completed all six ordered events:

1. `resolution_recorded`;
2. `project_binding_ready`;
3. `delivery_started`;
4. `delivery_applied`;
5. `reference_applied`; and
6. `capture_completed`.

The result was:

- recovery state `completed`;
- delivery ID `01a0e33f-0e57-79e3-8d65-3e2b44e2f801`;
- target WorkStateCommit `01a0e33f-2f2a-7c73-985e-6bb7b8f7695d`;
- canonical Record ID `01a0e33f-2efd-73a2-93d3-4bae9949dd24`;
- canonical Record version ID
  `01a0e33f-2efd-73a2-93d3-4bb818b7d51c`; and
- canonical Record version digest
  `8e5928a30bea616f46ff13209e67023cca28afb17c326f4c64163a4ff0ec46ca`.

Direct readback at the exact target commit returned the expected active
Finding and the same version digest. The statement explicitly says that this
is a new `cognition_v2` CaptureGroup replacing the route, not a mutation or
upgrade of the terminal legacy Capture.

## Cross-project and replay proof

Read-only recall from the Hernes ProjectRef returned exactly one completed
association pinned to the canonical Record version above, with
`store_opened=false` and `store_written=false`. The Hernes semantic Store
SHA-256 remained byte-identical at
`f3f2cdc3c8ed6f842ddf20ec9b81f89a440ccf6d72f354bcd691b350081cd644`.

The registry SHA-256 remained
`4e81852143651e698787c9dc06b5dbd9531d09583cac3e3a16ede592b96e8db8`.
Only the primary work-governance Store changed, from
`61b5afd4536194fc724c9f9b98735db822bccb9344a627485e3ca2d871a22889`
to
`3a56aa1016d3bd7d24c1948653a15dd88fb18e71432610abbd5f366908947bac`.
The resulting stored projection SHA-256 is
`7906c753e79729e453bda5b03610d77650aa8035e658a02933d7048e868cf937`.

A second recovery apply used fresh current digests and returned the same
commit and Record reference with:

- `projection_write_outcome=reused`;
- `target_delivery_reused=true`;
- all delivery, reference, completion, registry, binding, projection, and
  Store write flags false; and
- the primary and secondary Store hashes unchanged from the first result.

Finally, the earlier legacy Capture still reported four events and
`legacy_manifest_upgrade_required`; both of its recorded SHA-256 values were
unchanged. The evidence therefore proves replacement by a new intent without
rewriting old authority, one canonical semantic write, one immutable
cross-project association, and no silent duplicate on replay.

## Remaining boundaries

The accepted initial ProjectRef feature is now implemented, installed, and
bounded-live-canary verified. This does not activate automatic per-turn
capture, claim complete cognition observation, authorize rollback after v2
use, backfill historical associations, or publish/deploy the repository.
Those remain separate product or operational decisions.
