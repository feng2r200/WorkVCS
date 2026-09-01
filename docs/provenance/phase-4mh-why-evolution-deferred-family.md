# Phase 4MH Why Evolution Deferred Family Evidence

Date: 2026-09-01

## Scope

Phase 4MH adds a narrow `why` disclosure for anchored evolution:

```text
deferred_relation_family.0=evolution
```

The disclosure appears when the queried current Entity is an Entity causal
anchor for a ChangeSet reachable from the target commit through first-parent
history. It remains separate from stored relation edges and Handoff scope links.

## Implementation Evidence

- `crates/workvcs-core/src/history/why.rs` now computes
  `WhyQueryResult.deferred_relation_families` from first-parent-reachable
  ChangeSet causal anchors.
- `crates/workvcs-cli/src/main.rs` keeps using the existing `why` renderer,
  which already prints `deferred_relation_families` and indexed
  `deferred_relation_family.N` lines.
- The focused CLI test proves a causal Finding reports `evolution`, relation
  filters and limits do not remove that deferred family, and the superseded
  prior Decision in the same workflow reports zero deferred families.

## Focused Validation

PASS: `/tmp/workvcs-4mh-focused-validation-rerun3-20260901T073543Z`.

Covered checks:

```text
cargo fmt --all -- --check
cargo test -p workvcs-cli cli_supersedes_decision_record_atomically
cargo clippy -p workvcs-core --all-targets --all-features -- -D warnings
```

PASS: `/tmp/workvcs-4mh-medium-fix-validation-20260901T074818Z`.

Covered checks after independent-review medium fixes:

```text
git diff --check
cargo fmt --all -- --check
cargo test -p workvcs-cli cli_supersedes_decision_record_atomically
cargo clippy -p workvcs-core --all-targets --all-features -- -D warnings
```

## Dogfood Evidence

PASS: `/tmp/workvcs-4mh-why-evolution-dogfood-rerun-20260901T074712Z`.

Target project:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Target file:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
```

Key observations:

```text
target_status_unchanged=true
status_count_before=39
status_count_after=39
target_status_before=/tmp/workvcs-4mh-why-evolution-dogfood-rerun-20260901T074712Z/target-status-before.txt
target_status_after=/tmp/workvcs-4mh-why-evolution-dogfood-rerun-20260901T074712Z/target-status-after.txt
target_status_before_raw=/tmp/workvcs-4mh-why-evolution-dogfood-rerun-20260901T074712Z/target-status-before.z
target_status_after_raw=/tmp/workvcs-4mh-why-evolution-dogfood-rerun-20260901T074712Z/target-status-after.z
status_hash_before=dc1ebb780d895e7fc662ef4280425a9ff390bc37560c5a23545af23fb3037e03
status_hash_after=dc1ebb780d895e7fc662ef4280425a9ff390bc37560c5a23545af23fb3037e03
status_raw_hash_before=25db244655994ac854f760c60e5d7adb943fdd2324a21df698eb6188a8521198
status_raw_hash_after=25db244655994ac854f760c60e5d7adb943fdd2324a21df698eb6188a8521198
finding_id=01a05bef-c69e-7d50-96f7-86876a03b7d9
prior_decision_id=01a05bef-c6b4-7f70-943a-159b11939954
replacement_decision_id=01a05bef-c6cc-7b50-a7c9-f3b3622cd978
commit_id=01a05bef-c6e3-7a81-8bed-a1d44b30151e
changeset_id=01a05bef-c6e3-7a81-8bed-a1c5b1b3ae79
why_finding_deferred_family=evolution
why_filtered_deferred_family=evolution
why_prior_deferred_families=0
```

This proves the deferred-family disclosure against a real external project
scenario without mutating the target project.

## Independent Review

The independent review first found two medium issues:

```text
reachability wording needed to say first-parent history
dogfood needed raw before/after target status snapshots
```

Both were fixed. Re-review found no remaining blocker/high/medium findings.

## Final Validation

PASS: `/tmp/workvcs-4mh-final-validation-rerun-20260901T075035Z`.

Covered checks:

```text
git diff --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
scripts/smoke-v0.1-cli-workflow.sh
```

Superseded matrix evidence:
`/tmp/workvcs-4mh-final-validation-20260901T074018Z` passed before independent
review fixes and is retained only as intermediate evidence.

## Remaining Open

- Full evolution traversal remains Open.
- Epistemic deferred-family disclosure remains Open.
- Broader causal traversal and write-mode external-project dogfood remain Open.
