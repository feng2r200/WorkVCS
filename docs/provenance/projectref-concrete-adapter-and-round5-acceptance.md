# ProjectRef Concrete Adapter and Round-5 Acceptance Evidence

Status: Round-5 source evidence retained; acceptance ledger extended for ADR-0516
Date: 2026-09-27; matrix extension 2026-10-06
Authority: [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md),
[ADR-0516](../decisions/adr/0516-projectref-plan-durable-operation-routing.md),
[ProjectRef control plane v2](../architecture/projectref-control-plane-v2.md), and
[migration and acceptance contract](../architecture/projectref-registry-v2-migration-and-acceptance.md)

## Authorized boundary

This round adds the first concrete tool adapter behind the generic locator
interface, aligns the WorkVCS Skill and operator material, accounts for every
accepted matrix row, obtains independent adversarial review, and refreshes the
real-registry preview under a zero-write probe. It does not install the source
candidate, migrate or roll back the real registry, create a live journal or
activation marker, write a live semantic Store, activate a global Hook, stage
or commit Git state, push, tag, release, or deploy.

The 2026-10-06 ledger extension adds M-40 through M-42 as source/fixture proof
for typed Plan durable-operation routing. It does not rewrite the historical
Round-5 live-mutation boundary; installed-binary and live-adoption evidence is
kept in the dedicated ADR-0516 provenance record.

## Concrete adapter and omission closure

The core remains tool-neutral. `UnifiedLocatorInput` combines direct provider
invocations, already-verified semantic evidence, verified Git common-directory
evidence, and verified CWD evidence under one bounded input and the accepted
rank order. No core control-plane source file names Codex or ChatGPT.

The CLI integration layer provides `codex-app-project-metadata/v1` through
`--locator-adapter-context FILE`. The strict, command-local envelope contains
an adapter ID and bounded context. Its concrete context accepts:

- a canonical Codex home, used only to derive a stable installation namespace;
- optional task Project metadata verified from both project inventory and
  thread metadata, producing authoritative semantic evidence; and
- optional canonical `.chatgpt-projects/<project-id>` mirror evidence,
  producing separately explainable verified-derived evidence.

Metadata and mirror agreement retain both evidence records. A disagreement
keeps authoritative metadata primary and emits `context_mismatch`; it does not
create an alias. A valid context with neither input represents provider
unavailability and permits generic Git/CWD fallback. A supplied malformed,
secret-bearing, unsupported, noncanonical, or unverifiable assertion fails
closed. Raw context, thread ID, verification-source payload, and explanation
JSON are not journaled or written into a semantic Store.

The isolated end-to-end test
`cli_codex_project_adapter_routes_mirror_capture_to_semantic_owner_end_to_end`
starts from an otherwise unbound Project mirror, uses the exact migrated
semantic locator, resolves the Project rather than the mirror CWD, admits one
intent, delivers to the original semantic target through the explicit fixture
recovery path, proves replay idempotency, and verifies that the registry and
persisted journal omit the raw trusted-handoff fields.

The same audit found two accepted matrix behaviors that previously existed
only as prose. The core now exposes an explicit stronger-locator attachment
operation. An unclaimed semantic/repository locator can promote one selected
provisional ProjectRef without changing its binding, and exact replay is a
no-change reuse. A locator already claimed by another ProjectRef returns the
stable `locator_already_claimed` code without reassignment. There is still no
automatic ProjectRef merge.

## Matrix status vocabulary

- `source-proven`: focused automated evidence covers the source candidate or
  isolated fixture. It does not claim installation, activation, or live use.
- `inspection-proven`: bounded repository or documentation inspection is the
  appropriate evidence because the row asserts absence or wording.
- `review-pending`: independent review has not yet returned; this status must
  be replaced before round-5 closeout.
- `review-proven`: the independent adversarial reviewer found no unresolved
  issue within the round-5 boundary.
- `platform-deferred`: the source fails closed, but positive platform support
  remains outside the accepted roadmap and cannot be claimed.

The ledger intentionally does not convert source proof into live proof. The
six remaining roadmap rounds retain local commit, installation, migration,
read activation, journal activation, and live delivery as separate gates.

## Complete acceptance ledger

### Resolution and ownership

| ID | Status | Focused evidence |
| --- | --- | --- |
| R-01 | source-proven | `resolver_orders_explicit_semantic_repository_and_cwd` |
| R-02 | source-proven | `resolver_orders_explicit_semantic_repository_and_cwd` |
| R-03 | source-proven | `metadata_and_mirror_agree_and_remain_separately_explainable` |
| R-04 | source-proven | `authoritative_metadata_wins_a_conflicting_mirror_with_mismatch_diagnostic` |
| R-05 | source-proven | `resolver_orders_explicit_semantic_repository_and_cwd` |
| R-06 | source-proven | `resolver_orders_explicit_semantic_repository_and_cwd` |
| R-07 | source-proven | `cli_v2_unbound_semantic_locator_blocks_bound_cwd_fallback` |
| R-08 | source-proven | `first_write_registry_convergence_creates_once_and_rejects_target_substitution`; `cli_capture_recovery_delivers_primary_once_and_reuses_receipt` |
| R-09 | source-proven | `repository_and_cwd_first_writes_preserve_maturity_and_creation_provenance` |
| R-10 | source-proven | `repository_and_cwd_first_writes_preserve_maturity_and_creation_provenance` |
| R-11 | source-proven | `cli_v2_unbound_semantic_locator_blocks_bound_cwd_fallback` |
| R-12 | source-proven | `resolver_blocks_lower_fallback_and_fails_closed_on_conflict` |
| R-13 | source-proven | `stronger_locator_attachment_promotes_without_moving_target_and_reuses_exact_key` |
| R-14 | source-proven | `stronger_locator_attachment_rejects_another_projects_claim_without_change` |
| R-15 | source-proven | `two_semantic_projects_sharing_repository_and_display_name_remain_distinct`; `cli_project_ensure_keeps_same_named_projects_in_distinct_stores` |
| R-16 | source-proven | `unavailable_metadata_degrades_and_explicit_bad_context_fails_closed` |
| R-17 | source-proven | `resolver_blocks_lower_fallback_and_fails_closed_on_conflict` |

### Capture durability and recovery

| ID | Status | Focused evidence |
| --- | --- | --- |
| C-01 | source-proven | `round5_value_gate_and_no_record_policy_probe` in `validate-projectref-acceptance-matrix.sh` |
| C-02 | source-proven | `journal_conflict_and_preinstall_failures_leave_no_extra_intent` |
| C-03 | source-proven | `immutable_events_rebuild_a_byte_equivalent_projection_and_replay_without_duplicates` |
| C-04 | source-proven | `capture_recovery_conflict_records_status_without_bootstrap_or_fallback` |
| C-05 | source-proven | `commit_before_receipt_recovery_reuses_one_primary_result` |
| C-06 | source-proven | `capture_group_faults_preserve_primary_and_retry_only_missing_control_plane_steps` |
| C-07 | source-proven | `journal_atomically_installs_and_idempotently_reuses_intent`; `cli_capture_recovery_delivers_primary_once_and_reuses_receipt` |
| C-08 | source-proven | `journal_conflict_and_preinstall_failures_leave_no_extra_intent` |
| C-09 | source-proven | `capture_group_rejects_a_second_primary_role_before_admission` |
| C-10 | source-proven | `pending_references_retry_only_missing_and_secondary_recall_is_authoritative` |
| C-11 | source-proven | `cli_links_record_derived_from_source_record` |
| C-12 | source-proven | `capture_intent_rejects_secrets_oversize_and_invalid_group_before_write` |
| C-13 | source-proven | `stale_legacy_manifest_is_detected_without_rewriting_the_intent`; `stale_legacy_manifest_requires_upgrade_without_target_store_write` |
| C-14 | source-proven | `capture_recovery_faults_converge_forward_without_duplicate_identity`; `capture_group_faults_preserve_primary_and_retry_only_missing_control_plane_steps` |
| C-15 | source-proven | `cli_codex_project_adapter_routes_mirror_capture_to_semantic_owner_end_to_end` |
| C-16 | source-proven | `cli_capture_group_recovery_installs_reference_and_recalls_from_secondary_project` |
| C-17 | source-proven | `unresolved_group_adds_one_canonical_member_from_matching_locator_evidence` |
| C-18 | source-proven | `immutable_events_rebuild_a_byte_equivalent_projection_and_replay_without_duplicates` |
| C-19 | source-proven | `first_write_registry_convergence_creates_once_and_rejects_target_substitution`; `repository_and_cwd_first_writes_preserve_maturity_and_creation_provenance` |
| C-20 | source-proven | `capture_recovery_conflict_records_status_without_bootstrap_or_fallback` |
| C-21 | source-proven | `capture_recovery_faults_converge_forward_without_duplicate_identity` |
| C-22 | source-proven | `cli_capture_recovery_delivers_primary_once_and_reuses_receipt` |
| C-23 | source-proven | `event_gap_reordering_and_payload_tampering_fail_closed` |
| C-24 | source-proven | `registry_revalidation_preserves_receipt_only_for_the_exact_target` |
| C-25 | source-proven | `invalid_delivery_transition_is_rejected_before_event_install`; `capture_group_canonical_record_is_preflighted_before_target_write` |
| C-26 | source-proven | `cli_v2_journal_admission_is_default_off_exactly_activated_and_store_free` |
| C-27 | source-proven | `capture_recovery_terminalizes_semantically_invalid_manifest_without_target_write`; `capture_recovery_delivers_handoff_derived_from_finding` |

### Migration, rollback, activation, and admission

| ID | Status | Focused evidence |
| --- | --- | --- |
| M-01 | source-proven | `cli_project_registry_migration_preview_is_repeatable_json_capable_and_readonly`; refreshed real-state zero-write probe below |
| M-02 | source-proven | `cli_project_registry_migration_preview_reports_invalid_store_without_writes` |
| M-03 | source-proven | `registry_migration_apply_rejects_stale_digests_and_conflicting_artifacts_before_replace` |
| M-04 | source-proven | `cli_project_registry_migration_apply_installs_verified_v2_and_readonly_rollback_probe` |
| M-05 | source-proven | `cli_project_registry_migration_preview_reports_shared_target_without_aliasing`; `migration_candidate_preserves_targets_and_persists_one_to_one_receipts` |
| M-06 | source-proven | `migration_preview_reports_unknown_identity_invalid_target_and_duplicate_identity` |
| M-07 | source-proven | `registry_migration_apply_pre_rename_faults_preserve_v1_and_clean_candidate_temp` |
| M-08 | source-proven | `registry_migration_apply_post_rename_faults_are_indeterminate_and_never_auto_rollback` |
| M-09 | source-proven | `registry_migration_apply_lock_contention_preserves_v1_without_backup` |
| M-10 | source-proven | `registry_rollback_restores_exact_v1_and_reentry_is_verified_noop` |
| M-11 | source-proven | `registry_rollback_blocks_activation_journal_and_digest_drift_before_snapshot` |
| M-12 | source-proven | `cli_v1_project_read_routes_report_migration_required_and_remain_readonly` |
| M-13 | source-proven | `migration_candidate_preserves_targets_and_persists_one_to_one_receipts` |
| M-14 | source-proven | `cli_project_registry_migration_repair_preview_is_explicit_repeatable_and_readonly` |
| M-15 | source-proven | `cli_project_registry_migration_repair_preview_rejects_drift_without_writes` |
| M-16 | source-proven | `cli_project_registry_migration_repair_preview_rejects_drift_without_writes` |
| M-17 | source-proven | `migration_ownership_repair_manifest_is_strict_bounded_and_registry_bound` |
| M-18 | source-proven | `cli_project_registry_migration_repair_preview_is_explicit_repeatable_and_readonly` |
| M-19 | source-proven | `migration_ownership_repair_replaces_identity_preserves_target_and_retires_history` |
| M-20 | source-proven | `cli_project_registry_migration_repair_does_not_bypass_invalid_target` |
| M-21 | source-proven | `cli_project_registry_migration_repair_rejects_store_under_historical_git_repository_root` |
| M-22 | source-proven | `registry_rollback_with_configured_home_blocks_sidecar_activation_alias` |
| M-23 | source-proven | `routing_activation_post_install_faults_are_indeterminate_and_recover_by_status` |
| M-24 | source-proven | `cli_v2_routing_activation_rejects_malformed_stale_and_symlink_markers` |
| M-25 | source-proven | `cli_v2_cwd_durable_write_matrix_fails_closed_without_registry_or_store_changes` |
| M-26 | source-proven | `cli_v2_routing_activation_uses_configured_home_marker_path` |
| M-27 | source-proven | `registry_rollback_pre_rename_faults_preserve_v2_and_clean_candidate_temp` |
| M-28 | source-proven | `registry_rollback_post_rename_faults_are_indeterminate_and_probe_recovers_state` |
| M-29 | source-proven | `registry_rollback_restores_exact_v1_and_reentry_is_verified_noop` |
| M-30 | source-proven | `registry_rollback_blocks_activation_journal_and_digest_drift_before_snapshot`; `registry_rollback_rejects_noncanonical_symlinked_and_conflicting_snapshots` |
| M-31 | source-proven | `registry_rollback_with_configured_home_blocks_sidecar_activation_alias` |
| M-32 | source-proven | `journal_admission_first_persists_intent_and_forces_rollback_to_fail_closed`; `registry_rollback_first_restores_v1_and_admission_persists_nothing` |
| M-33 | platform-deferred | `require_atomic_registry_replace_support` inspection proves fail-closed behavior; positive non-Unix support and fault evidence remain deferred |
| M-34 | source-proven | `orphan_journal_quiescence_lock_blocks_rollback_until_exact_fixture_recovery` |
| M-35 | source-proven | `cli_v1_routed_capture_is_target_neutral_and_survives_migration` |
| M-36 | source-proven | `cli_v2_journal_admission_is_default_off_exactly_activated_and_store_free` |
| M-37 | source-proven | `journal_admission_activation_faults_are_indeterminate_and_recover_by_status` |
| M-38 | source-proven | `cli_v2_journal_admission_is_default_off_exactly_activated_and_store_free`; `cli_codex_project_adapter_routes_mirror_capture_to_semantic_owner_end_to_end` |
| M-39 | source-proven | `cli_capture_recovery_delivers_primary_once_and_reuses_receipt` |
| M-40 | source-proven | `cli_v2_plan_admit_and_evolve_use_one_durable_journal`; `cli_v2_plan_retry_after_delivery_started_reports_created_target`; `cli_v2_plan_conflict_is_journaled_without_store_write`; `cli_v2_plan_validation_failure_is_terminal_without_store_write`; `cli_v2_plan_receipt_size_is_preflighted_before_store_write`; `typed_intents_reject_cross_family_receipts_and_failures`; `capture_event_limit_accepts_limit_minus_one_and_limit_but_rejects_limit_plus_one` |
| M-41 | source-proven | `legacy_journal_marker_requires_explicit_same_snapshot_plan_capability_refresh`; `journal_admission_activation_candidate_is_separate_and_exact_snapshot_bound` |
| M-42 | source-proven | `cli_v2_plan_commit_before_receipt_recovers_by_idempotent_replay` |

### Boundaries and non-regression

| ID | Status | Focused evidence |
| --- | --- | --- |
| N-01 | source-proven | `cli_v2_read_routes_are_default_off_and_digest_activated_only_in_fixture`; `cli_codex_project_adapter_routes_mirror_capture_to_semantic_owner_end_to_end` |
| N-02 | source-proven | `cli_project_binding_uses_git_common_dir_identity_across_worktrees` |
| N-03 | source-proven | `two_semantic_projects_sharing_repository_and_display_name_remain_distinct` |
| N-04 | source-proven | `cli_project_registry_migration_repair_does_not_bypass_invalid_target`; `capture_recovery_conflict_records_status_without_bootstrap_or_fallback` |
| N-05 | source-proven | `pending_references_retry_only_missing_and_secondary_recall_is_authoritative` |
| N-06 | source-proven | `unified_adapter_input_rejects_provider_spoofing_nonsemantic_output_and_bad_explanation`; `cli_codex_project_adapter_routes_mirror_capture_to_semantic_owner_end_to_end` |
| N-07 | source-proven | `cargo test --workspace --all-targets --locked` result below |
| N-08 | inspection-proven | `round5_no_global_hook_repository_inspection` result below |
| N-09 | inspection-proven | `validate-projectref-acceptance-matrix.sh`; operator-recovery maturity validation result below |
| N-10 | review-proven | Independent adversarial review found no unresolved code or contract defect; evidence-placeholder finding was closed by the results below |
| N-11 | source-proven | `cli_codex_project_adapter_routes_mirror_capture_to_semantic_owner_end_to_end`; `round5_value_gate_and_no_record_policy_probe` |
| N-12 | inspection-proven | `round5_durability_claim_wording_probe` in `validate-projectref-acceptance-matrix.sh` |

## Validation and review results

The final source candidate passed:

- `cargo test -p workvcs-cli locator_adapter::tests --locked`: `4 passed`;
- `cargo test -p workvcs-core --test projectref_control_plane_foundation
  --locked`: `15 passed`;
- `cargo test -p workvcs-core --test projectref_adapter_migration_preview
  --locked`: `10 passed`;
- `cargo test -p workvcs-cli
  cli_codex_project_adapter_routes_mirror_capture_to_semantic_owner_end_to_end
  --locked`: `1 passed`;
- `cargo test --workspace --all-targets --locked`: passed, including all 260
  CLI tests and every core integration target;
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: passed;
- `cargo fmt --all -- --check` and `git diff --check`: passed;
- `scripts/validate-schema-v0.1.sh`: passed;
- `scripts/validate-projectref-acceptance-matrix.sh`: 98 exact unique rows,
  valid statuses, aligned adapter docs, value/durability wording, and a
  tool-neutral core all passed;
- `scripts/smoke-v0.1-cli-workflow.sh`: `smoke_result=passed`; and
- `scripts/operator-recovery-maturity-v0.1.sh`:
  `phase4ni_operator_recovery_maturity=PASS`, with all 54 core error codes plus
  the CLI parse code covered by the operator guide.

The independent adversarial reviewer inspected the cumulative source and
authority changes without editing files. It found no high, medium, or low code
defect in the generic/core separation, concrete-adapter validation and
persistence boundary, command routing, migration/activation guards,
stronger-locator attachment, or M-33 classification. Its one high-priority
evidence finding was that this section and N-10 were still placeholders while
the parent's probes were running. The results in this section close that
finding. The reviewer could not run Cargo through its default toolchain because
the host Xcode license is not accepted; the primary validation used the
existing Command Line Tools SDK/compiler/linker explicitly and made no system
change.

The refreshed real-state observation used the current source-tree binary and
the locally retained exact Hernes repair material. The repository does not
retain its private path or Project identifiers. In one observation window the
probe hashed the complete configured WorkVCS control-plane tree, ran two
default previews and two repair-aware previews, then hashed the tree again.
Results:

| Field | Before | After |
| --- | --- | --- |
| control-plane entries | `59` | `59` |
| complete tree snapshot SHA-256 | `6602e5f8dbd750e0264af6e1d5ec6aca40b2875c16bcfe5953dd1584ad6db8c3` | `6602e5f8dbd750e0264af6e1d5ec6aca40b2875c16bcfe5953dd1584ad6db8c3` |
| registry SHA-256 | `c57ca8aa9b1f4a7d3ff0b1db9a35384b7e3776f809b7fb44737498a0ee323ed4` | same |
| registry metadata | mode `0644`, size `2862`, mtime/ctime `1789695294`, inode `107118324` | same |
| Hernes Store SHA-256 | `f3f2cdc3c8ed6f842ddf20ec9b81f89a440ccf6d72f354bcd691b350081cd644` | same |
| Hernes Store metadata | mode `0644`, size `712704`, mtime/ctime `1789625522`, inode `106869852` | same |

Both default previews were byte-identical. They reported source digest
`blake3-256:acb09e62afae0949eadba491039a0f65c387018f25c97e05136ec42d4b989175`,
preview digest
`blake3-256:86865a63f5e0eb8588eaa40e5abcd08498c051b43a25d571bbcc6c37486625c7`,
six mappings, `apply_eligible=false`, and explicit
`read_only=true/registry_written=false/journal_written=false/store_written=false`.

Both repair-aware previews were byte-identical. They retained the same source
digest, repair-manifest digest
`blake3-256:cb4a392116a56778113c5b4ceb63ce2830fd707c360915eb7c68756cfb01dd2f`,
and project-metadata evidence digest
`blake3-256:497ce279e0c9388eb3b8690d9c0f9859c302b1be3ce0d0d10d5a20e552af9bd1`.
They reported preview digest
`blake3-256:e327969821c8e85a247f44c82a853ce48224596a9be0d030e926fa6d09d65893`,
six mappings, one ownership repair, `apply_eligible=true`, and the same four
explicit no-write fields. Eligibility remains evidence only; no apply was run.

The home-root and registry-sidecar read-activation markers,
journal-admission markers, capture-journal roots, and canonical registry
journal-quiescence lock were absent before and after. No backup, candidate,
snapshot, marker, journal, ProjectRef, or Store object was created. The
temporary external repair-manifest file was removed after the probe. Governance
closeout records written after this observation window are separate from and
do not weaken the migration-preview zero-write claim.

Matrix disposition is therefore 88 `source-proven`, three
`inspection-proven`, one `review-proven`, and one `platform-deferred` row.

## Explicit deferrals and remaining gates

- M-33 remains the only deferred matrix row: non-Unix apply/rollback support
  has a source fail-closed guard but no positive atomic-replacement or fault
  evidence.
- No round-5 evidence claims that the candidate is committed, installed, or
  selected by the current runtime.
- No real registry migration/rollback, read activation, journal activation,
  semantic Store delivery, secondary-reference delivery, or Hook activation
  has occurred.
- Rounds 6 through 11 remain the exact local-commit, installation, migration,
  read-canary, admission-canary, and delivery-canary confirmation gates.
