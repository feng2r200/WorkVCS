# Authorized Existing-Binding Delivery Local Adoption Evidence

Status: Stage C local adoption complete; selected Stage D reconciliation completed separately
Date: 2026-10-08
Decision: [ADR-0519](../decisions/adr/0519-authorized-existing-binding-delivery-and-operation-inventory.md)
Plan: [Authorized Existing-Binding Delivery Design and Change Plan](authorized-existing-binding-delivery-plan.md)
Source candidate: [Authorized Existing-Binding Delivery Candidate Evidence](authorized-existing-binding-delivery-candidate.md)

## Scope and authority

This evidence closes the authorized Stage C boundary: package the exact
reviewed source, atomically replace the user-local binary and installed Skill,
prove package/install parity, and run one newly admitted same-binding canary
through semantic readback and zero-write replay. Stage C did not authorize or
perform activation-marker refresh, historical backlog recovery, Push, tag,
release, remote deployment, rollback, or destructive cleanup.

Stage B implementation commit
`c89a75a21b4f84dda6215b565511edf444a78541` had already passed the full
source validation recorded by the candidate evidence. Evidence-closure commit
`0bbd6425b9d06b342784be441f4af254916e5cd6` was the exact clean package source.
Documentation-only correction commit
`6b1874b465342b3b614a28a17cfa3ac48ab0f35c` subsequently corrected candidate
validation attribution; it did not change the packaged binary or Skill.
Independent review closed that correction with no remaining Blocker, High, or
Medium issue before the canary was run.

## Exact package and installed parity

The repository packaging workflow reported `source_git_dirty=false` and built:

- package directory:
  `target/package/workvcs-aarch64-apple-darwin-0bbd6425b9d0-20261008T082849Z-92068`;
- archive SHA-256:
  `761e54ece594d3f03f0d86b7b89b4506bb51892ba9db95498e5446b0dab77026`;
- manifest SHA-256:
  `78027d96814464971e843eeba6060b9469e8455e626ef9640528b69127c8427e`;
- packaged and installed binary SHA-256:
  `280cb0456795c2d439fbc29e9f4b0d6f6b88daae970c1a6a4eeba340325958bb`;
- installed Skill entry SHA-256:
  `321389a79322b33a678b123116fe699bfb19e0bc4e99a8b8ef16d4fcef5e38f7`;
- seven-file Skill tree manifest SHA-256:
  `77f1fb43b38d32f0e41846c4fbafffc4fef191010501fa9a30141b087a08beda`.

Byte comparison proved the packaged binary equals
`$HOME/.local/bin/workvcs`. Verification of the packaged Skill manifest from
`$HOME/.agents/skills/workvcs` reported all seven files `OK`. Installation used
the repository's explicit packaging/install path; no alternate artifact or
second installation mechanism was introduced.

## Control-plane preconditions and byte stability

Before and after installation, `project health` reported:

- `health=healthy`, `read_only=true`;
- registry revision `3` and semantic digest
  `blake3-256:b5ec12377cbe536bce8cb53e5378c95d019a5c8dde1584843b843e68868aa6c9`;
- seven of seven bindings valid and 49 local content objects verified;
- read routing active;
- journal admission active at version 2 with cognition capture, Plan admit,
  and Plan evolve capabilities active; and
- the WorkVCS source checkout resolved to ProjectRef
  `01a0e2e6-f2e6-77a2-b858-7680d26d9eb0`.

The registry and activation-marker file SHA-256 values were unchanged across
installation and canary replay:

| Artifact | SHA-256 |
| --- | --- |
| ProjectRef registry | `e30b185dfa61071b4e6083eee86a570836b112e76f3e2e2d1be07bb2b3a0455f` |
| Read-routing activation | `5c93245fbaf790bee2ad677d06ef566937448f9efcf594470512382aa2fd6a96` |
| Journal-admission activation | `84132fa9610cac33327d9d7ef3d3fbed7d902a2f00e7c6bee6f1464a078c3b66` |

Current markers already satisfied the installed contract. No activation
refresh was required or performed.

## Bounded same-binding canary

The installed binary admitted one new `cognition_v2` Finding with idempotency
key `workvcs-adr0519-stage-c-canary-20261008-0bbd642` and the explicit
`--deliver-existing-binding` option. The first invocation reported:

- `capture_status=completed`, `admission_outcome=created`;
- CaptureId `01a11aa4-8f83-77a3-832d-1fb379bdc920` and payload digest
  `blake3-256:1e98a2f72aa4f57c85df9ea02a2fed865b881a4b54b20319b05ac21f8e8d6c26`;
- DeliveryId `01a11aa4-920e-75d4-ae8a-deded64e7769`;
- target WorkVCS commit `01a11aa4-931a-7307-ac60-98b743a5bc3f`;
- `journal_written=true`, `store_written=true`, and
  `projection_written=true`; and
- `registry_written=false`, `project_ref_created=false`,
  `delivery_status=completed`, and `recovery_action=none`.

Read-only status reconstructed four immutable events, a current projection
with digest
`blake3-256:44875848388dc73581e500c988a4d48c69c0c643dad3e45d3a428bfd8e773508`,
the same receipt and target commit, and no recovery action. Exact semantic
readback returned one active Finding:

- RecordId `01a11aa4-92d7-7434-9244-4b6f658c35d4`;
- RecordVersionId `01a11aa4-92d7-7434-9244-4b70f59ec428`;
- Record state digest
  `52c738c54944dc01ff9750ac7b5932366d3bcff144bd2fd19e42c3eb404a2ea2`;
- statement: “The installed ADR-0519 candidate completed one bounded
  same-binding Stage C canary from journal admission through a verified target
  receipt and idempotent replay.”

The target Branch ended at the named commit with state digest
`7a5ed6e1f4fa3faae9396314932e4b8ff7c8dd86bfbae19d7dc9b6d9b279858a`.

## Exact replay evidence

Repeating the same installed command and idempotency key returned:

- `admission_outcome=reused` and the same CaptureId;
- the same DeliveryId and target commit;
- `journal_written=false`, `store_written=false`, and
  `projection_written=false`; and
- `target_delivery_reused=true`, `delivery_status=completed`, and
  `recovery_action=none`.

Before and after replay, the registry, both activation markers, intent,
projection, all four event files, and target SQLite Store retained the same
SHA-256 values. The target Branch retained the same commit and state digest.
The immutable intent SHA-256 was
`0ce0edd37e392c198e08ab71b50e8833b62766bddf7f726d93ffd2fba85ceb36`,
the projection file SHA-256 was
`ed4f01f1a5b678d71919fbad65dc9c6c34aeb9ae93776ef38922b438ff58cf84`,
and the target Store SHA-256 was
`18d4c69c9118aca334041b2d3ade82ad994a20004521f253959539f0db60543f`.

## Remaining boundary

Stage C proves local installed adoption for a newly admitted eligible capture;
it did not grant standing authority over older open operations. A later,
separately authorized Stage D selected and reconciled exactly two historical
operations under fresh status evidence; see
[Authorized Existing-Binding Delivery Historical Reconciliation Evidence](authorized-existing-binding-delivery-historical-reconciliation.md).
No previously admitted Capture was recovered by Stage C itself. Tag, release,
deployment, rollback, history rewrite, and destructive cleanup remain outside
the completed local boundaries.
