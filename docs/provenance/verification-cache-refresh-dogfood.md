# Verification Cache Refresh Dogfood Evidence

Status: Phase 4KZ local dogfood evidence
Recorded: 2026-09-01

This file records the evidence for the explicit Verification applicability cache
refresh path added by ADR-0415.

The implemented recovery path is deliberately narrow:

1. a caller identifies one Work Branch and one Verification;
2. Core resolves the current branch head;
3. Core derives observed Resource stamps from the Verification resource basis
   only when every Resource basis entry has a baseline Resource Observation;
4. Core delegates branch-head checking, WorkState basis checking, Resource
   fingerprint comparison, and cache persistence to the existing
   `record_verification_applicability` semantics; and
5. the CLI reports the evaluated commit, applicability, reason code, and stamp
   count needed by `cache-show`, AC status, and handoff flows.

Validated command surface:

```text
workvcs verification cache-refresh STORE \
  --branch BRANCH_ID \
  --verification VERIFICATION_ENTITY_ID \
  --expected-evaluated-commit CURRENT_HEAD \
  --expected-applicability applicable \
  --expected-reason-code all_basis_applicable \
  --expected-resource-stamps 1
```

When `--expected-evaluated-commit` is supplied, it is checked before cache
refresh. A mismatch returns a branch-head conflict and leaves the previous cache
row in place.

Targeted validation:

```text
cargo test -q -p workvcs-core --test verify_wrapper_phase4ky
result: passed, 6 tests

cargo test -q -p workvcs-cli
result: passed
```

Repository smoke validation:

```text
scripts/smoke-v0.1-cli-workflow.sh
smoke_result=passed
store_id=01a0589b-bbfe-7830-adbd-b0d1c9c6f8f2
workspace_id=01a0589b-c1b7-7d22-8b18-c4dec7cf344c
branch_id=01a0589b-c1b7-7d22-8b18-c50241cb1c54
task_entity_id=01a0589b-c46d-7ba3-a535-c669e3af8e2e
verification_entity_id=01a0589c-2839-7432-bc9a-768ddab08927
```

The smoke workflow now uses `verification cache-refresh` for the post-head-advance
recovery step that previously required manual `verification cache-record` stamp
arguments.

Residual gaps:

- The refresh command does not observe external Resources.
- Resource adapter contracts, path/glob normalization, and automatic refresh
  remain outside this slice.
- Durable handoff dogfood remains the next higher-value V1 loop to close.
