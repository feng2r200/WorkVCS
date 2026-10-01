# ProjectRef Capture Semantic Preflight and Terminal Invalid Candidate

Date: 2026-10-01
Base commit: `c1be6847ab3f9e7a0ba0462de8e6d27e0bf89fe7`
Status: implementation and live-recovery evidence validated; installed to the
user-local WorkVCS binary and Skill, used to terminalize one live historical
invalid Capture, and used to admit and deliver its corrected replacement. This
evidence grants no Push, release, or deployment authority.

## Problem evidence

The public ProjectRef-v2 `capture` route decoded a canonical
`CognitionCaptureManifest` but did not run its full Record and relation
semantics before installing an immutable `CaptureIntent`. The Store delivery
path performed that validation later. A payload containing
`Finding --supports--> Handoff` could therefore be admitted and then fail as
non-retryable `record_invalid` after `delivery_started`.

The domain rule itself was consistent: `supports` accepts a Finding source and
a Decision or Knowledge target. A coordination Handoff derived from a Finding
uses `Handoff --derived_from--> Finding` when explicit provenance is useful.

The recovery projection had no terminal state for this deterministic failure,
so the immutable intent appeared to remain `pending_primary` even though exact
replay could not succeed.

## Candidate correction

- `CognitionCaptureManifest::validate_semantics` now shares the same
  target-neutral semantic preparation used by Store delivery and normalizes
  capture-content failures to `record_invalid`.
- Routed `capture` invokes that validator before project resolution,
  activation checks, journal creation, registry mutation, or Store access.
- Recovery validates a materialized historical manifest before opening the
  target Store for mutation.
- A historical deterministic failure records
  `failure_code=semantic_manifest_invalid` and derives the terminal state of
  the same name with
  `recovery_action=start_new_capture_with_corrected_payload`.
- The original intent remains immutable. Replay reuses the terminal event and
  the corrected content must use a new Capture.

The accepted decision is
[ADR-0515](../decisions/adr/0515-capture-semantic-preflight-and-terminal-invalid-intent.md).

## Isolated failure and recovery evidence

Focused fixtures prove:

1. An active ProjectRef-v2 journal route rejects
   `Finding --supports--> Handoff` as `record_invalid` before the journal root
   exists. Registry bytes and all target SQLite files remain unchanged.
2. `Handoff --derived_from--> Finding` reaches `completed` and produces exactly
   one `derived_from` relation.
3. A historical invalid intent is interrupted immediately after durable
   `delivery_started`, reproducing the live three-event `pending_primary`
   shape. A second digest-locked apply records only
   `semantic_manifest_invalid`; the target Store and intent bytes remain
   unchanged.
4. A third apply writes no event and performs no target delivery.

## Validation evidence

The local source candidate passed:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo test --workspace --all-targets --locked`;
- `scripts/validate-schema-v0.1.sh`;
- `scripts/smoke-v0.1-cli-workflow.sh`; and
- `scripts/validate-projectref-acceptance-matrix.sh`, including all 95 exact
  contract/ledger rows and the semantic preflight/terminal-invalid probe.

The focused admission, valid-provenance delivery, historical-invalid
terminalization, delivery-start fault, and idempotent replay tests all passed.

## Installed canary and live recovery evidence

After separate user authorization, the repository packaging path installed the
candidate at the existing user-local binary and Skill locations. The packaged
and installed binary SHA-256 was
`de94900320f04bb9fb10b7b31f1c96be152caeaeaad6aace5f954cb7b470213f`.
The installed six-file Skill tree matched the packaged manifest with SHA-256
`1be30ed0482638e62d3c9475c8bc93a4cd7cfff783a6f99319c5f021beac535b`.
The previous binary and Skill tree were retained in a local rollback backup
before replacement.

The installed binary then read Capture
`01a0f6b6-477b-779c-8167-08c37768ea74` as a three-event
`pending_primary` projection with registry digest
`blake3-256:608fbd17ca0469019062653c546bf8c8b56b66c9e5c0f1bad8b39a33fb87da64`
and projection digest
`blake3-256:a3e8a1ac8ac4860c89f5f9680daf020394d3f662963e0632e7934faf8fca9352`.
One apply guarded by those exact digests added only the terminal failure event
and current projection. Its result reported:

- `recovery_state=semantic_manifest_invalid`;
- `delivery_failure_code=semantic_manifest_invalid`;
- `delivery_failure_written=true`;
- `target_delivery_written=false`; and
- `target_commit_id=none`.

The target Branch remained at commit
`01a0f6eb-2fe0-74e2-a050-3769e79e6ab5` with state digest
`1468b3b0d17c19749d80b7a8fa67f22aa865f11cdde1b17701bbcd871efc7fe4`.
The immutable intent retained SHA-256
`3cde9c86e0dfed29f30a0e6adec7a9e7a359757c406e81e4e2c8c6f535438e96`.
Status then reported four events, a current
`semantic_manifest_invalid` projection, and
`recovery_action=start_new_capture_with_corrected_payload`.

A second apply using the fresh terminal projection digest
`blake3-256:43d35c4b85e87cfaad36e25fe47e019ea9116dd0f90d6c12b4c228082b4f7215`
reused the projection, retained exactly four events and the same target Branch
head, and reported every event, registry, projection, and target-delivery write
flag as false. This is bounded evidence for the installed route and this exact
Capture, not a general release or deployment claim.

## Corrected replacement evidence

The terminal source intent was reconstructed without changing its two Record
statements, scopes, Knowledge, or Evidence. The replacement used idempotency key
`work-governance-activation-replay-20261001-v2-corrected` and replaced the
invalid relation with exactly one
`Handoff --derived_from--> Finding` relation. The generated manifest had
SHA-256 `31a6290cf1fa38efe6d3e099da8b0a52de10fc6786ee24cedf6b32f817a5d446`.

Installed Admission created Capture
`01a0f6f6-6d72-702e-ac88-eabbd8f20e6b` with no Store write. Repeating the same
manifest reused that Capture and wrote no second intent. Read-only recovery
status reported zero events, `pending_project`, and projection digest
`blake3-256:64dcbe5e4cce470e3b6df3fef6b0149b244ba9b0e1f4416068032e1c8918d323`.

One apply guarded by that projection digest and the unchanged registry digest
completed the recovery. It installed four control-plane events and created one
target Store commit:

- target commit `01a0f6f8-51d6-75be-a631-524b83845349`;
- Finding `01a0f6f8-51a3-76ba-8e9a-24a89a1d02a0`;
- Handoff `01a0f6f8-51a3-76ba-8e9a-24aa4cc9b84d`; and
- `derived_from` relation `01a0f6f8-51a3-76ba-8e9a-24acdddede11`, from that
  Handoff to that Finding.

Exact Record readback preserved the original statements and scopes. A filtered
relation query returned exactly one matching relation. Status reported four
events, `completed`, a current projection, and `recovery_action=none`.

A second apply using terminal projection digest
`blake3-256:4c3f99fc10635e7f97e4590ee363287aa16dab4c7cf36a9f99ce55efe9bc9ab4`
reused the same target commit and projection. It retained four events and
reported `target_delivery_written=false`, `target_delivery_reused=true`, and
all control-plane write flags false. The source Capture remains independently
terminal as `semantic_manifest_invalid` with its original four-event history.

The restored Handoff is a historical gap-fill under its original recorded
scope. Newer WorkVCS Handoffs govern the current execution boundary.

## Remaining authority boundary

The local binary and Skill are installed, the historical invalid Capture is
terminal, and one corrected replacement is delivered and verified. No registry
or activation marker changed. Required-participation WorkVCS governance
checkpoints are normal Store records and remain distinct from Capture recovery
delivery. The source delivery is governed independently from installation and
recovery; Push, release, and deployment remain separate, unauthorized actions.
