# Phase 5H Local Object v2 Cutover Evidence

Date: 2026-09-17
Status: completed local cutover

## Scope and authority

This slice corrected the mismatch between WorkVCS's BLAKE3-256 content digest
and the legacy `sha256` local object directory name. The user authorized local
implementation, validation, migration of the three configured Stores, global
installation, recoverable cleanup, local commits, and local fast-forward
integration. Push, tag, release, deployment, and other remote mutation were not
authorized.

The accepted boundary is recorded in
[ADR-0512](../decisions/adr/0512-local-object-v2-cutover.md): v1 was permitted
only as bounded migration input and creates no permanent reader, writer,
fallback locator, command, or compatibility obligation.

## Delivery shape

- Commit `133bd34ed513e487fd0546c4f531c51112966abe` introduced the v2
  runtime contract and an isolated one-time migration candidate.
- Commit `7855910add90b2cfff6a1f80841704f9bb7b0051` removed the migration
  binary, migration API, v1 loader, legacy locator helper, and migration-only
  tests from the final source.
- The final contract is `object_store_format_version=2`, backend
  `workvcs.local-object-v2`, and locator
  `.workvcs-objects/<store-id>/blake3-256/<prefix>/<digest>`.
- Store format and SQLite schema versions remain `1`.
- A final source scan found no v1 backend, `sha256` object locator, or migration
  API in `crates/` or `scripts/`. ADR-0512 retains the historical vocabulary
  solely to explain the completed cutover.

## Candidate proof before live mutation

SQLite-consistent copies of all three configured Stores and their exact legacy
object directories were migrated outside the live registry. The candidate
proved:

- object counts `0`, `3`, and `9` respectively;
- digest and size verification before database cutover;
- successful v2 open and local-object validation after cutover;
- a second apply classified every copy as already current and created no
  second migration;
- Evidence extraction returned a 4,237-byte object with digest
  `0a9b43e4d8ae35ceb8ec4405d06a88dc30a2d1a9c096ee04b85820d7898a0dc8`;
- Bundle profile v2 exported and validated commit
  `01a0ade3-1574-74c0-8a28-f8b7d80a214f` with 990 payload files,
  2,133 references, and 8 portable Evidence contents;
- manifest digest
  `e90ba42c23bfe70f79a35d8131bc2edd0427f3f8f07d3bacd0dbe4fa6a97f02b`
  and payload-index digest
  `f6a474413c0c9dd6f972571ffab83982c4d671ae2ee314a5df6fd7a89b14cee7`
  were stable under validation.

This confirms that Bundle v2 did not require a profile migration: it carries
verified object bytes and derives the target-local locator during apply.

## Live recovery boundary

Before live mutation, all Stores passed the installed v1 integrity check and
the migration candidate's exact-count preflight. SQLite online backups passed
`PRAGMA integrity_check` with these SHA-256 file digests:

- Hernes: `35a50b61c281d7acfc503dde0c4b5a0d02031fa443f9d91aee1706ba4bfc4c11`
- irp_stocklens: `4ec79caeb13f37e9daacc3cb594d4595ed64575491781e146272c0a61d28b6f2`
- shared WorkVCS/work-governance:
  `d58a2a0199feb6e151232926216d98bdb76cd8efeb976ace1b4479b96ce6bc4c`

The legacy object directories were copied with the backups. No SQLite WAL or
SHM sidecars and no running WorkVCS process were present at the cutover gate.

## Live migration results

All three migrations completed and recorded canonical Store migration
outcomes:

| Store | Local objects | Migration id |
| --- | ---: | --- |
| Hernes | 0 | `01a0adfe-598e-7550-aded-906b3d58fa62` |
| irp_stocklens | 3 | `01a0adfe-59b8-7571-b7e6-bc78665dcb5b` |
| shared WorkVCS/work-governance | 9 | `01a0adfe-59fb-78e2-8d10-0ce1e1520c91` |

Each outcome records object-store version `1` to `2`, outcome `applied`, and
that source objects were retained through the validation gate.

The v2-only release binary was then installed at
`/Users/example/.local/bin/workvcs` with SHA-256
`e73186179186ecc87f7adc40cbed187e1d959f7afba7113d512495213597f6d0`.
The packaged WorkVCS Skill was installed at `/Users/example/.agents/skills/workvcs`;
its entry digest is
`0ee1a145294fd283a1001f070d94445617d673088e6209ae9fe36934303e8d88`
and its five-file tree digest is
`4856a216817f420b4a422c6a842e188087b776ca4e25403a9e7f1d10b460e1b0`.

## Final validation and cleanup

The installed binary then proved:

- all three Stores report object-store version `2` and digest algorithm
  `blake3-256`;
- all three Stores pass integrity validation;
- all 12 local objects extract with their expected BLAKE3-256 digest;
- all four registry bindings discover successfully;
- the live shared Store repeats the Bundle v2 export and validation proof
  above.

Only after those checks, the two existing legacy directories containing 3 and
9 objects were removed from active storage and moved recoverably to
`/Users/example/.Trash/workvcs-local-object-v1-cutover-20260917-0615`. A post-cleanup
pass again opened every Store, passed integrity validation, and extracted all
12 v2 objects. Active WorkVCS storage contains zero `sha256` object directories.

Repository validation passed:

- `cargo test --workspace`;
- the complete CLI smoke workflow;
- focused migration, Store compatibility, Evidence, and Bundle tests;
- `cargo clippy --workspace --all-targets -- -D warnings` with the repository's
  existing `too_many_arguments` lint exception;
- `cargo fmt --all -- --check`;
- `scripts/validate-schema-v0.1.sh`.

No push, tag, release, deployment, or remote mutation was performed.
