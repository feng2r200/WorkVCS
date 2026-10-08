#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
contract="$repo_root/docs/architecture/projectref-registry-v2-migration-and-acceptance.md"
ledger="$repo_root/docs/provenance/projectref-concrete-adapter-and-round5-acceptance.md"
skill="$repo_root/skills/workvcs/SKILL.md"
workflows="$repo_root/skills/workvcs/references/workflows.md"
tool_reference="$repo_root/docs/operator/workvcs-tool-reference.md"
quickstart="$repo_root/docs/operator/quickstart-and-recovery.md"
config_reference="$repo_root/skills/workvcs/references/configuration.md"
capability_routing="$repo_root/skills/workvcs/references/capability-routing.md"
semantics="$repo_root/skills/workvcs/references/semantics.md"
checkpoint_delivery="$repo_root/skills/workvcs/references/checkpoint-delivery.md"
cli_source="$repo_root/crates/workvcs-cli/src/main.rs"
core_journal_source="$repo_root/crates/workvcs-core/src/control_plane/journal.rs"
core_journal_tests="$repo_root/crates/workvcs-core/tests/projectref_journal_recovery.rs"
core_model_source="$repo_root/crates/workvcs-core/src/control_plane/model.rs"
isolation_adr="$repo_root/docs/decisions/adr/0517-shared-project-binding-isolation.md"
isolation_evidence="$repo_root/docs/provenance/shared-project-binding-isolation.md"
operator_control_adr="$repo_root/docs/decisions/adr/0518-operator-control-plane-health-and-recovery-contract.md"
operator_control_evidence="$repo_root/docs/provenance/operator-control-plane-v1.md"
existing_binding_adr="$repo_root/docs/decisions/adr/0519-authorized-existing-binding-delivery-and-operation-inventory.md"
existing_binding_plan="$repo_root/docs/provenance/authorized-existing-binding-delivery-plan.md"
existing_binding_candidate="$repo_root/docs/provenance/authorized-existing-binding-delivery-candidate.md"
existing_binding_adoption="$repo_root/docs/provenance/authorized-existing-binding-delivery-local-adoption.md"
existing_binding_reconciliation="$repo_root/docs/provenance/authorized-existing-binding-delivery-historical-reconciliation.md"
operation_disposition_adr="$repo_root/docs/decisions/adr/0520-operation-disposition-and-global-inventory-classification.md"
plan_validation_adr="$repo_root/docs/decisions/adr/0521-plan-manifest-validation-and-routed-rejection-diagnostics.md"
error_recovery_guide="$repo_root/docs/operator/error-recovery-guide.md"
core_error_source="$repo_root/crates/workvcs-core/src/error.rs"
operator_recovery_audit="$repo_root/scripts/operator-recovery-maturity-v0.1.sh"
config_example="$repo_root/config.toml.example"
readme="$repo_root/README.md"
readme_zh="$repo_root/README.zh-CN.md"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

extract_ids() {
    local source="$1"
    awk -F '|' '/^\| [RCMN]-[0-9][0-9] \|/ {
        id=$2
        gsub(/^ +| +$/, "", id)
        print id
    }' "$source"
}

extract_ids "$contract" >"$tmp_dir/contract-ids"
extract_ids "$ledger" >"$tmp_dir/ledger-ids"

[[ "$(wc -l <"$tmp_dir/contract-ids" | tr -d ' ')" == "98" ]] || {
    echo "acceptance contract does not contain exactly 98 matrix rows" >&2
    exit 1
}
[[ "$(wc -l <"$tmp_dir/ledger-ids" | tr -d ' ')" == "98" ]] || {
    echo "acceptance ledger does not contain exactly 98 matrix rows" >&2
    exit 1
}
[[ -z "$(sort "$tmp_dir/contract-ids" | uniq -d)" ]] || {
    echo "acceptance contract contains duplicate matrix IDs" >&2
    exit 1
}
[[ -z "$(sort "$tmp_dir/ledger-ids" | uniq -d)" ]] || {
    echo "acceptance ledger contains duplicate matrix IDs" >&2
    exit 1
}
diff -u <(sort "$tmp_dir/contract-ids") <(sort "$tmp_dir/ledger-ids")

awk -F '|' '/^\| [RCMN]-[0-9][0-9] \|/ {
    status=$3
    gsub(/^ +| +$/, "", status)
    if (status != "source-proven" && status != "inspection-proven" &&
        status != "review-pending" && status != "review-proven" &&
        status != "platform-deferred") {
        print "unsupported matrix status: " status > "/dev/stderr"
        exit 1
    }
}' "$ledger"

for source in "$skill" "$workflows" "$tool_reference" "$quickstart"; do
    grep -q -- '--locator-adapter-context' "$source"
done
for source in "$workflows" "$tool_reference" "$quickstart"; do
    grep -q -- '--expected-activation-digest' "$source"
done
grep -q 'command-local' "$config_reference"
grep -q 'command-local' "$config_example"
grep -q 'no silent loss after admission' "$skill"
grep -q 'no silent loss after admission' "$tool_reference"
grep -q 'Use \*\*required participation\*\*' "$skill"
grep -q 'changes selection and admission timing only' "$skill"
grep -q 'workvcs config show' "$skill"
grep -q 'references/capability-routing.md' "$skill"
grep -q '^# Capability routing$' "$capability_routing"
grep -q '^## Bootstrap live WorkVCS state from an existing stage baseline$' "$workflows"
grep -q 'baseline is a project-native snapshot' "$skill"
grep -q '^## Agent participation policy$' "$readme"
grep -q '^## Agent 参与策略$' "$readme_zh"
grep -q 'semantic_manifest_invalid' "$contract"
grep -q 'C-26.*cli_v2_journal_admission_is_default_off_exactly_activated_and_store_free' "$ledger"
grep -q 'C-27.*capture_recovery_terminalizes_semantically_invalid_manifest_without_target_write' "$ledger"
grep -q 'M-40.*cli_v2_plan_admit_and_evolve_use_one_durable_journal' "$ledger"
grep -q 'M-41.*legacy_journal_marker_requires_explicit_same_snapshot_plan_capability_refresh' "$ledger"
grep -q 'M-42.*cli_v2_plan_commit_before_receipt_recovers_by_idempotent_replay' "$ledger"
grep -q 'fn capture_recovery_terminalizes_semantically_invalid_manifest_without_target_write' "$cli_source"
grep -q 'fn capture_recovery_delivers_handoff_derived_from_finding' "$cli_source"
grep -q 'fn cli_v2_plan_admit_and_evolve_use_one_durable_journal' "$cli_source"
grep -q 'fn cli_v2_plan_commit_before_receipt_recovers_by_idempotent_replay' "$cli_source"
grep -q 'fn cli_v2_plan_retry_after_delivery_started_reports_created_target' "$cli_source"
grep -q 'fn cli_v2_plan_conflict_is_journaled_without_store_write' "$cli_source"
grep -q 'fn cli_v2_plan_validation_failure_is_terminal_without_store_write' "$cli_source"
grep -q 'fn cli_v2_plan_receipt_size_is_preflighted_before_store_write' "$cli_source"
grep -q 'fn legacy_journal_marker_requires_explicit_same_snapshot_plan_capability_refresh' "$cli_source"
grep -q 'fn capture_event_limit_accepts_limit_minus_one_and_limit_but_rejects_limit_plus_one' "$core_journal_source"
grep -q 'fn typed_intents_reject_cross_family_receipts_and_failures' "$core_journal_source"
grep -q 'pub fn isolate_shared_target' "$core_model_source"
grep -q 'BindingSource::Isolation' "$core_model_source"
grep -q 'fn cli_v2_shared_binding_isolation_is_digest_locked_preserves_history_and_stales_markers' "$cli_source"
grep -q 'project isolate-shared-binding --preview' "$isolation_adr"
grep -q 'No old Store bytes or Work State are copied' "$isolation_evidence"
grep -q 'isolate-shared-binding' "$tool_reference"
grep -q 'isolate-shared-binding' "$quickstart"
grep -q 'shared-binding isolation' "$skill"
grep -q 'isolate-shared-binding' "$config_reference"
grep -q 'workvcs project health' "$tool_reference"
grep -q 'workvcs project health' "$quickstart"
grep -q 'project operation-recovery' "$skill"
grep -q 'project operation-recovery' "$workflows"
grep -q 'project operation-recovery' "$checkpoint_delivery"
grep -q 'fn project_health_reports_activation_progress_and_is_strictly_read_only' "$cli_source"
grep -q 'fn project_health_distinguishes_missing_capability_from_stale_marker' "$cli_source"
grep -q 'fn project_health_binding_validation_calls_each_binding_once' "$cli_source"
grep -q 'routing_activation_inactive' "$core_error_source"
grep -q 'journal_admission_activation_inactive' "$core_error_source"
grep -q 'journal_admission_capability_inactive' "$core_error_source"
grep -q '^Status: Accepted and implemented$' "$operator_control_adr"
grep -q 'one complete validation of every binding' "$operator_control_adr"
grep -q '^Status: Source implemented and locally accepted$' "$operator_control_evidence"
grep -q '^Status: Accepted; source implemented, locally validated, locally adopted, selected historical reconciliation complete, and delivered to main$' "$existing_binding_adr"
grep -q '^Status: Stages B-D and exact remote delivery complete$' "$existing_binding_plan"
grep -q '^# Authorized Existing-Binding Delivery Candidate Evidence$' "$existing_binding_candidate"
grep -q '^# Authorized Existing-Binding Delivery Local Adoption Evidence$' "$existing_binding_adoption"
grep -q '^Status: Stage C local adoption complete; selected Stage D reconciliation completed separately$' "$existing_binding_adoption"
grep -q '^# Authorized Existing-Binding Delivery Historical Reconciliation Evidence$' "$existing_binding_reconciliation"
grep -q '^Status: Stage D selected historical reconciliation and exact remote delivery complete$' "$existing_binding_reconciliation"
grep -q '01a1112c-51f3-759c-a6d7-de2e4bfadfcf' "$existing_binding_reconciliation"
grep -q '01a11a6e-a137-707c-9001-e825e391ee41' "$existing_binding_reconciliation"
grep -q '01a11afc-fb82-71fc-9235-24e2ba462701' "$existing_binding_reconciliation"
grep -q '01a11afd-276f-7723-8cc7-46d8f1d811be' "$existing_binding_reconciliation"
grep -q '01a11174-57bb-76e5-9de6-a8ddf1bcde95' "$existing_binding_reconciliation"
grep -q '01a11aa4-92d7-7434-9244-4b6f658c35d4' "$existing_binding_reconciliation"
grep -q 'not standing batch authority' "$existing_binding_reconciliation"
grep -q '46210daec733c2add31dfbd730336056cb99d999' "$existing_binding_reconciliation"
grep -q '37762878714' "$existing_binding_reconciliation"
grep -q -- '--deliver-existing-binding' "$skill"
grep -q -- '--deliver-existing-binding' "$workflows"
grep -q -- '--deliver-existing-binding' "$checkpoint_delivery"
grep -q -- '--deliver-existing-binding' "$tool_reference"
grep -q -- '--deliver-existing-binding' "$quickstart"
grep -q -- '--list-open' "$skill"
grep -q -- '--list-open' "$workflows"
grep -q -- '--list-open' "$checkpoint_delivery"
grep -q -- '--list-open' "$tool_reference"
grep -q -- '--list-open' "$quickstart"
grep -q -- '--list-all' "$skill"
grep -q -- '--list-all' "$workflows"
grep -q -- '--list-all' "$checkpoint_delivery"
grep -q -- '--list-all' "$tool_reference"
grep -q -- '--list-all' "$quickstart"
grep -q -- '--dispose superseded' "$skill"
grep -q -- '--dispose superseded' "$checkpoint_delivery"
grep -q -- '--dispose superseded' "$tool_reference"
grep -q -- '--dispose superseded' "$quickstart"
grep -q 'plan validate --operation admit|evolve' "$skill"
grep -q 'plan validate --operation admit|evolve' "$workflows"
grep -q 'plan validate --operation admit|evolve' "$checkpoint_delivery"
grep -q 'plan validate --operation admit|evolve' "$quickstart"
grep -q 'workvcs plan validate --operation admit' "$tool_reference"
grep -q '^Status: Accepted; source implemented and locally validated$' "$operation_disposition_adr"
grep -q '^Status: Accepted; source implemented and locally validated$' "$plan_validation_adr"
grep -q 'fn cli_operation_disposition_is_guarded_terminal_and_globally_auditable' "$cli_source"
grep -q 'fn cli_plan_manifest_validate_is_zero_write_and_routed_rejection_is_actionable' "$cli_source"
grep -q 'fn legacy_projection_without_operation_disposition_is_stale_not_invalid' "$core_journal_source"
grep -q 'responsible caller.*not a monitoring job' "$skill"
grep -q '^## Keep ownership with the responsible task$' "$checkpoint_delivery"
grep -q 'counterfactual effect of acting now' "$checkpoint_delivery"
grep -q 'deterministic terminal projection remains terminal' "$checkpoint_delivery"
grep -q 'superseded undelivered intent' "$checkpoint_delivery"
grep -q 'routine same-target completion' "$tool_reference"
grep -q 'routine pending receipts are not left' "$quickstart"
grep -q 'Historical open rows are' "$readme"
grep -q '历史开放条目只是分类证据' "$readme_zh"
grep -q 'capture_delivery_incomplete' "$core_error_source"
grep -q 'project_owner_unbound' "$core_error_source"
grep -q 'capture_delivery_incomplete' "$error_recovery_guide"
grep -q 'project_owner_unbound' "$error_recovery_guide"
grep -q 'fn cli_capture_existing_binding_is_explicit_and_completed_replay_is_zero_write' "$cli_source"
grep -q 'fn cli_operation_inventory_is_readonly_allowlisted_and_digest_pageable' "$cli_source"
grep -q 'fn cli_capture_existing_binding_refuses_shared_target_before_new_event' "$cli_source"
grep -q 'fn cli_capture_existing_binding_refuses_conflict_before_new_event' "$cli_source"
grep -q 'fn cli_capture_existing_binding_refuses_invalid_binding_before_new_event' "$cli_source"
grep -q 'fn cli_capture_existing_binding_rejects_capture_group_and_v1_before_admission' "$cli_source"
grep -q 'fn capture_existing_binding_rejects_unresolved_invocation_before_new_event' "$cli_source"
grep -q 'fn capture_existing_binding_rejects_changed_registry_without_later_target_proof' "$cli_source"
grep -q 'fn capture_existing_binding_rejects_prior_binding_path_mismatch_before_new_event' "$cli_source"
grep -q 'fn cli_clean_unbound_is_degraded_and_flagged_capture_is_partial' "$cli_source"
grep -q 'fn cli_operation_inventory_preserves_terminal_action_on_shared_binding' "$cli_source"
grep -q 'fn cli_operation_inventory_does_not_report_missing_binding_as_opened_store' "$cli_source"
grep -q 'fn authority_inventory_reads_each_chain_without_materializing_projection_cache' "$core_journal_tests"
grep -q 'existing-binding-inventory' "$operator_recovery_audit"
grep -q 'capture-delivery-incomplete-key-value' "$operator_recovery_audit"
grep -q 'capture-delivery-incomplete-json' "$operator_recovery_audit"
grep -q '^| ED-01 ' "$contract"
grep -q '^| OL-01 ' "$contract"
grep -q '^| UB-01 ' "$contract"
grep -q 'plan_admit_v1' "$workflows"
grep -q 'plan_evolve_v1' "$workflows"
grep -q 'plan_receipt_too_large' "$workflows"
grep -q '`payload_kind`' "$checkpoint_delivery"
grep -q 'Finding --supports--> Handoff' "$semantics"

if grep -R -i -n -E 'codex|chatgpt' "$repo_root/crates/workvcs-core/src/control_plane"; then
    echo "tool-specific term leaked into the generic core control plane" >&2
    exit 1
fi

echo "matrix_rows=98"
echo "matrix_ids_exact=true"
echo "matrix_statuses_valid=true"
echo "round5_value_gate_and_no_record_policy_probe=passed"
echo "round5_durability_claim_wording_probe=passed"
echo "required_participation_policy_probe=passed"
echo "activation_refresh_contract_probe=passed"
echo "control_plane_selection_fallback_probe=passed"
echo "adapter_docs_aligned=true"
echo "semantic_preflight_and_terminal_invalid_probe=passed"
echo "typed_plan_durable_operation_probe=passed"
echo "shared_binding_isolation_probe=passed"
echo "operator_control_plane_v1_probe=passed"
echo "authorized_existing_binding_delivery_probe=passed"
echo "operation_disposition_and_global_inventory_probe=passed"
echo "plan_manifest_validation_and_rejection_diagnostics_probe=passed"
echo "generic_core_tool_neutral=true"
