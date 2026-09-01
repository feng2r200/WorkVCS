#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/workvcs-operator-recovery.${timestamp}.XXXXXX")"
log_dir="$repo_root/.work-governance/runtime/logs/phase-4ni/operator-recovery-maturity.${timestamp}"
mkdir -p "$log_dir"

store="$tmp_dir/operator-recovery.sqlite"
project_dir="$tmp_dir/project"
scoped_file="$project_dir/src/lib.rs"
mkdir -p "$(dirname "$scoped_file")"
printf 'baseline local file\n' >"$scoped_file"

die() {
    printf 'operator_recovery_error=%s\n' "$*" >&2
    printf 'log_dir=%s\n' "$log_dir" >&2
    printf 'tmp_dir=%s\n' "$tmp_dir" >&2
    exit 1
}

step() {
    printf 'operator_recovery_step=%s\n' "$*" >&2
}

write_log() {
    local name="$1"
    local content="$2"
    printf '%s\n' "$content" >"$log_dir/${name}.out"
}

append_command_log() {
    local name="$1"
    local status="$2"
    shift 2
    {
        printf 'step=%s\n' "$name"
        printf 'status=%s\n' "$status"
        printf 'argv='
        printf '%q ' "$@"
        printf '\n\n'
    } >>"$log_dir/commands.log"
}

value() {
    local output="$1"
    local key="$2"
    local line
    local prefix="${key}="
    while IFS= read -r line; do
        if [[ "$line" == "$prefix"* ]]; then
            printf '%s\n' "${line#"$prefix"}"
            return 0
        fi
    done <<<"$output"
    die "missing key ${key}"
}

expect_value() {
    local output="$1"
    local key="$2"
    local expected="$3"
    local actual
    actual="$(value "$output" "$key")"
    [[ "$actual" == "$expected" ]] || die "${key} expected ${expected} got ${actual}"
}

expect_nonempty() {
    local output="$1"
    local key="$2"
    local actual
    actual="$(value "$output" "$key")"
    [[ -n "$actual" && "$actual" != "none" ]] || die "${key} is empty"
}

expect_contains() {
    local output="$1"
    local needle="$2"
    [[ "$output" == *"$needle"* ]] || die "missing output fragment ${needle}"
}

capture_success() {
    local name="$1"
    local __var="$2"
    shift 2
    local output
    local status
    set +e
    output="$("$cli" "$@" 2>&1)"
    status=$?
    set -e
    write_log "$name" "$output"
    append_command_log "$name" "$status" "$cli" "$@"
    [[ "$status" -eq 0 ]] || die "${name} failed with status ${status}"
    printf -v "$__var" '%s' "$output"
}

capture_failure() {
    local name="$1"
    local __var="$2"
    shift 2
    local output
    local status
    set +e
    output="$("$cli" "$@" 2>&1)"
    status=$?
    set -e
    write_log "$name" "$output"
    append_command_log "$name" "$status" "$cli" "$@"
    [[ "$status" -ne 0 ]] || die "${name} unexpectedly succeeded"
    printf -v "$__var" '%s' "$output"
}

audit_error_guide_coverage() {
    local core_codes_file="$tmp_dir/core-error-codes.txt"
    local guide_codes_file="$tmp_dir/guide-error-codes.txt"
    local expected_codes_file="$tmp_dir/expected-error-codes.txt"
    local missing_file="$tmp_dir/missing-error-codes.txt"
    local extra_file="$tmp_dir/extra-error-codes.txt"
    local retry_file="$tmp_dir/guide-retryability.txt"

    sed -n '/impl ErrorCode {/,/impl fmt::Display for ErrorCode/p' \
        "$repo_root/crates/workvcs-core/src/error.rs" \
        | awk -F '"' '/Self::/ && /=>/ {print $2}' \
        | sort -u >"$core_codes_file"
    awk -F '`' '/^\| `[^`]+` \|/ {print $2}' \
        "$repo_root/docs/operator/error-recovery-guide.md" \
        | sort -u >"$guide_codes_file"
    {
        cat "$core_codes_file"
        printf 'cli_parse_error\n'
    } | sort -u >"$expected_codes_file"

    comm -23 "$expected_codes_file" "$guide_codes_file" >"$missing_file"
    comm -13 "$expected_codes_file" "$guide_codes_file" >"$extra_file"
    [[ ! -s "$missing_file" ]] || die "operator guide is missing error codes: $(tr '\n' ' ' <"$missing_file")"
    [[ ! -s "$extra_file" ]] || die "operator guide has stale extra error codes: $(tr '\n' ' ' <"$extra_file")"

    awk -F '|' '/^\| `[^`]+` \|/ {
        code=$2
        retryable=$4
        gsub(/[ `]/, "", code)
        gsub(/[ `]/, "", retryable)
        print code "=" retryable
    }' "$repo_root/docs/operator/error-recovery-guide.md" >"$retry_file"
    while IFS='=' read -r code retryable; do
        local expected="false"
        if [[ "$code" == "branch_head_conflict" ]]; then
            expected="true"
        fi
        [[ "$retryable" == "$expected" ]] || die "guide retryability for ${code} expected ${expected} got ${retryable}"
    done <"$retry_file"

    core_error_codes="$(wc -l <"$core_codes_file" | tr -d ' ')"
    guide_error_codes="$(wc -l <"$guide_codes_file" | tr -d ' ')"
    write_log "guide-coverage" "$(
        printf 'core_error_codes=%s\n' "$core_error_codes"
        printf 'guide_error_codes=%s\n' "$guide_error_codes"
        printf 'missing_in_guide=none\n'
        printf 'extra_in_guide=none\n'
        printf 'retryability_matches_core_rule=true\n'
    )"
}

step "build cli"
(cd "$repo_root" && cargo build -q -p workvcs-cli)
cli="$repo_root/target/debug/workvcs"
[[ -x "$cli" ]] || die "built CLI not found at ${cli}"

step "guide coverage audit"
audit_error_guide_coverage

step "cli parse JSON recovery"
capture_failure "cli-parse-json" parse_json_output --error-format json unknown-command
expect_contains "$parse_json_output" '"error_code":"cli_parse_error"'
expect_contains "$parse_json_output" '"error_category":"usage"'
expect_contains "$parse_json_output" '"retryable":false'
expect_contains "$parse_json_output" '"clap_error_kind":"invalid_subcommand"'
capture_success "cli-help-recovery" help_output --help
expect_contains "$help_output" 'Usage: workvcs'
expect_contains "$help_output" '--error-format <ERROR_FORMAT>'

step "store and workspace setup"
capture_success "init" init_output init "$store" --display-name "operator-recovery-maturity"
capture_success "workspace-create" workspace_output workspace create "$store" --display-name "operator-recovery-workspace"
workspace_id="$(value "$workspace_output" "workspace_id")"
branch_id="$(value "$workspace_output" "branch_id")"
head_commit_id="$(value "$workspace_output" "genesis_commit_id")"
expect_value "$workspace_output" "branch_name" "main"

step "branch head conflict recovery"
stale_head="$head_commit_id"
capture_success "branch-conflict-first-task" first_task_output \
    task create "$store" \
    --branch "$branch_id" \
    --head "$stale_head" \
    --description "Operator recovery first branch-head mutation" \
    --priority 1
head_commit_id="$(value "$first_task_output" "commit_id")"
capture_failure "branch-conflict-stale-head" branch_conflict_output \
    --error-format key-value \
    task create "$store" \
    --branch "$branch_id" \
    --head "$stale_head" \
    --description "Operator recovery stale branch-head mutation" \
    --priority 2
expect_value "$branch_conflict_output" "error_code" "branch_head_conflict"
expect_value "$branch_conflict_output" "error_category" "mutation"
expect_value "$branch_conflict_output" "retryable" "true"
capture_success "branch-head-refresh" branch_head_output branch head "$store" --branch "$branch_id"
refreshed_head="$(value "$branch_head_output" "head_commit_id")"
expect_value "$branch_head_output" "head_commit_id" "$head_commit_id"
capture_success "branch-conflict-retry" branch_retry_output \
    task create "$store" \
    --branch "$branch_id" \
    --head "$refreshed_head" \
    --description "Operator recovery retried branch-head mutation" \
    --priority 2
head_commit_id="$(value "$branch_retry_output" "commit_id")"
expect_nonempty "$branch_retry_output" "task_entity_id"

step "resource applicability recovery"
capture_success "resource-task-create" resource_task_output \
    task create "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --description "Operator recovery Resource-backed task" \
    --priority 3
resource_task_id="$(value "$resource_task_output" "task_entity_id")"
resource_task_version_id="$(value "$resource_task_output" "task_entity_version_id")"
head_commit_id="$(value "$resource_task_output" "commit_id")"

capture_success "resource-ac-create" resource_ac_output \
    ac create "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --task "$resource_task_id" \
    --task-version "$resource_task_version_id" \
    --local-key "AC-4NI-RESOURCE" \
    --statement "Operator recovery restores Resource applicability from stale, unavailable, and error states."
criterion_id="$(value "$resource_ac_output" "acceptance_criterion_entity_id")"
head_commit_id="$(value "$resource_ac_output" "commit_id")"

capture_success "resource-create" resource_output resource create "$store" --kind local-file
resource_id="$(value "$resource_output" "resource_id")"
expect_value "$resource_output" "resource_kind" "local-file"

capture_success "resource-baseline-observe" baseline_output \
    resource observe "$store" \
    --resource "$resource_id" \
    --adapter-kind local-file \
    --adapter-schema-version 1 \
    --content-file "$scoped_file"
baseline_observation_id="$(value "$baseline_output" "observation_id")"
baseline_fingerprint="$(value "$baseline_output" "fingerprint")"

scope_payload_json="$(printf '{"path":"%s"}' "$scoped_file")"
capture_success "resource-verification-record" verification_output \
    verification record "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --acceptance-criterion "$criterion_id" \
    --result passed \
    --method cli \
    --resource "$resource_id" \
    --adapter-kind local-file \
    --adapter-schema-version 1 \
    --scope-kind path \
    --scope-schema-version 1 \
    --scope-payload-json "$scope_payload_json" \
    --baseline-fingerprint "$baseline_fingerprint" \
    --baseline-observation "$baseline_observation_id"
verification_id="$(value "$verification_output" "verification_entity_id")"
head_commit_id="$(value "$verification_output" "commit_id")"

capture_success "resource-applicable-initial" resource_applicable_initial \
    verification cache-refresh "$store" \
    --branch "$branch_id" \
    --verification "$verification_id" \
    --resource-content-from-scope-path \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability applicable \
    --expected-reason-code all_basis_applicable \
    --expected-resource-stamps 1
expect_value "$resource_applicable_initial" "applicability" "applicable"
expect_value "$resource_applicable_initial" "reason_code" "all_basis_applicable"

printf 'changed local file\n' >"$scoped_file"
capture_success "resource-drift" resource_drift_output \
    verification cache-refresh "$store" \
    --branch "$branch_id" \
    --verification "$verification_id" \
    --resource-content-from-scope-path \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability stale \
    --expected-reason-code resource_drift \
    --expected-resource-stamps 1
expect_value "$resource_drift_output" "applicability" "stale"
expect_value "$resource_drift_output" "reason_code" "resource_drift"
capture_success "resource-drift-ac-status" resource_drift_ac_status ac status "$store" --branch "$branch_id" --criterion "$criterion_id"
expect_value "$resource_drift_ac_status" "status" "stale"

printf 'baseline local file\n' >"$scoped_file"
capture_success "resource-drift-recovery" resource_drift_recovery \
    verification cache-refresh "$store" \
    --branch "$branch_id" \
    --verification "$verification_id" \
    --resource-content-from-scope-path \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability applicable \
    --expected-reason-code all_basis_applicable \
    --expected-resource-stamps 1
expect_value "$resource_drift_recovery" "applicability" "applicable"
capture_success "resource-drift-recovery-ac-status" resource_drift_recovery_ac_status ac status "$store" --branch "$branch_id" --criterion "$criterion_id"
expect_value "$resource_drift_recovery_ac_status" "status" "verified"

rm "$scoped_file"
capture_success "resource-unavailable" resource_unavailable_output \
    verification cache-refresh "$store" \
    --branch "$branch_id" \
    --verification "$verification_id" \
    --resource-content-from-scope-path \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability unknown \
    --expected-reason-code resource_unavailable \
    --expected-resource-stamps 1
expect_value "$resource_unavailable_output" "applicability" "unknown"
expect_value "$resource_unavailable_output" "reason_code" "resource_unavailable"
capture_success "resource-unavailable-cache-show" resource_unavailable_cache \
    verification cache-show "$store" \
    --branch "$branch_id" \
    --verification "$verification_id"
expect_value "$resource_unavailable_cache" "resource_stamp.0.observation_status" "unavailable"
expect_value "$resource_unavailable_cache" "resource_stamp.0.observed_fingerprint" "none"
expect_value "$resource_unavailable_cache" "resource_stamp.0.observation_id" "none"

printf 'baseline local file\n' >"$scoped_file"
capture_success "resource-unavailable-recovery" resource_unavailable_recovery \
    verification cache-refresh "$store" \
    --branch "$branch_id" \
    --verification "$verification_id" \
    --resource-content-from-scope-path \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability applicable \
    --expected-reason-code all_basis_applicable \
    --expected-resource-stamps 1
expect_value "$resource_unavailable_recovery" "applicability" "applicable"

rm "$scoped_file"
mkdir "$scoped_file"
capture_success "resource-error" resource_error_output \
    verification cache-refresh "$store" \
    --branch "$branch_id" \
    --verification "$verification_id" \
    --resource-content-from-scope-path \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability unknown \
    --expected-reason-code resource_error \
    --expected-resource-stamps 1
expect_value "$resource_error_output" "applicability" "unknown"
expect_value "$resource_error_output" "reason_code" "resource_error"
capture_success "resource-error-cache-show" resource_error_cache \
    verification cache-show "$store" \
    --branch "$branch_id" \
    --verification "$verification_id"
expect_value "$resource_error_cache" "resource_stamp.0.observation_status" "error"
expect_value "$resource_error_cache" "resource_stamp.0.observed_fingerprint" "none"
expect_value "$resource_error_cache" "resource_stamp.0.observation_id" "none"

rmdir "$scoped_file"
printf 'baseline local file\n' >"$scoped_file"
capture_success "resource-error-recovery" resource_error_recovery \
    verification cache-refresh "$store" \
    --branch "$branch_id" \
    --verification "$verification_id" \
    --resource-content-from-scope-path \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability applicable \
    --expected-reason-code all_basis_applicable \
    --expected-resource-stamps 1
expect_value "$resource_error_recovery" "applicability" "applicable"
capture_success "resource-final-ac-status" resource_final_ac_status ac status "$store" --branch "$branch_id" --criterion "$criterion_id"
expect_value "$resource_final_ac_status" "status" "verified"

step "claim stale takeover recovery"
capture_success "claim-source-session" claim_source_session_output \
    session start "$store" \
    --workspace "$workspace_id" \
    --branch "$branch_id" \
    --expected-workspace "$workspace_id" \
    --expected-branch "$branch_id" \
    --expected-lifecycle-state active
claim_source_session_id="$(value "$claim_source_session_output" "session_id")"
capture_success "claim-taking-session" claim_taking_session_output \
    session start "$store" \
    --workspace "$workspace_id" \
    --branch "$branch_id" \
    --expected-workspace "$workspace_id" \
    --expected-branch "$branch_id" \
    --expected-lifecycle-state active
claim_taking_session_id="$(value "$claim_taking_session_output" "session_id")"
capture_success "claim-task" claim_task_output \
    claim task "$store" \
    --session "$claim_source_session_id" \
    --task "$resource_task_id" \
    --expected-mode exclusive \
    --expected-lifecycle-state active
claim_id="$(value "$claim_task_output" "claim_id")"
capture_success "claim-blocked-guard" claim_blocked_guard_output \
    claim guard "$store" \
    --session "$claim_taking_session_id" \
    --task "$resource_task_id" \
    --expected-allowed false \
    --expected-reason exclusive_claim_owned_by_other_session \
    --expected-active-claims 1
expect_value "$claim_blocked_guard_output" "allowed" "false"
expect_value "$claim_blocked_guard_output" "reason" "exclusive_claim_owned_by_other_session"
expect_value "$claim_blocked_guard_output" "stale_takeover_available" "true"
expect_value "$claim_blocked_guard_output" "stale_takeover_claim_id" "$claim_id"
expect_value "$claim_blocked_guard_output" "stale_takeover_previous_session_id" "$claim_source_session_id"
expect_value "$claim_blocked_guard_output" "stale_takeover_required_previous_session_lifecycle_state" "potentially_stale"

capture_failure "claim-takeover-before-stale" claim_takeover_before_stale \
    --error-format key-value \
    claim takeover "$store" \
    --session "$claim_taking_session_id" \
    --claim "$claim_id" \
    --force \
    --rationale "operator recovery matrix"
expect_value "$claim_takeover_before_stale" "error_code" "claim_invalid"
expect_value "$claim_takeover_before_stale" "error_category" "runtime"
expect_value "$claim_takeover_before_stale" "retryable" "false"
expect_contains "$claim_takeover_before_stale" "must be potentially_stale"

capture_success "claim-mark-stale" claim_mark_stale_output \
    session mark-stale "$store" \
    --session "$claim_source_session_id" \
    --rationale "operator recovery matrix: previous session cannot continue" \
    --expected-session "$claim_source_session_id" \
    --expected-lifecycle-state potentially_stale \
    --expected-active-workspace "$workspace_id" \
    --expected-active-branch "$branch_id"
expect_value "$claim_mark_stale_output" "lifecycle_state" "potentially_stale"
capture_success "claim-takeover-recovery" claim_takeover_output \
    claim takeover "$store" \
    --session "$claim_taking_session_id" \
    --claim "$claim_id" \
    --force \
    --rationale "operator recovery matrix" \
    --expected-previous-claim "$claim_id" \
    --expected-previous-session "$claim_source_session_id" \
    --expected-previous-session-lifecycle-state potentially_stale \
    --expected-session "$claim_taking_session_id" \
    --expected-mode exclusive \
    --expected-lifecycle-state active
claim_takeover_id="$(value "$claim_takeover_output" "claim_id")"
expect_value "$claim_takeover_output" "previous_lifecycle_state" "released"
expect_value "$claim_takeover_output" "previous_session_lifecycle_state" "potentially_stale"
expect_value "$claim_takeover_output" "lifecycle_state" "active"
capture_success "claim-takeover-guard" claim_takeover_guard_output \
    claim guard "$store" \
    --session "$claim_taking_session_id" \
    --task "$resource_task_id" \
    --expected-allowed true \
    --expected-reason owned_exclusive_claim \
    --expected-active-claims 1
expect_value "$claim_takeover_guard_output" "allowed" "true"
expect_value "$claim_takeover_guard_output" "reason" "owned_exclusive_claim"

step "merge unresolved guard recovery"
merge_base_commit_id="$head_commit_id"
capture_success "merge-source-branch" source_branch_output \
    branch fork "$store" \
    --from-branch "$branch_id" \
    --name "operator-recovery-source"
source_branch_id="$(value "$source_branch_output" "branch_id")"
expect_value "$source_branch_output" "head_commit_id" "$merge_base_commit_id"

capture_success "merge-target-task" target_merge_task_output \
    task create "$store" \
    --branch "$branch_id" \
    --head "$merge_base_commit_id" \
    --description "Operator recovery target merge marker" \
    --priority 4
target_merge_head_id="$(value "$target_merge_task_output" "commit_id")"
head_commit_id="$target_merge_head_id"

capture_success "merge-source-task" source_merge_task_output \
    task create "$store" \
    --branch "$source_branch_id" \
    --head "$merge_base_commit_id" \
    --description "Operator recovery source merge task" \
    --priority 5
source_merge_task_id="$(value "$source_merge_task_output" "task_entity_id")"
source_merge_head_id="$(value "$source_merge_task_output" "commit_id")"

capture_success "merge-start" merge_start_output \
    merge start "$store" \
    --target-branch "$branch_id" \
    --source-branch "$source_branch_id" \
    --session "$claim_taking_session_id" \
    --expected-runtime-state active \
    --expected-merge-base "$merge_base_commit_id" \
    --expected-target-head "$target_merge_head_id" \
    --expected-source-head "$source_merge_head_id" \
    --expected-origin-session "$claim_taking_session_id"
merge_id="$(value "$merge_start_output" "merge_id")"
capture_success "merge-show-unresolved" merge_show_output \
    merge show "$store" \
    --merge "$merge_id" \
    --expected-runtime-state active \
    --expected-outcome none
merge_item_id="$(value "$merge_show_output" "item.0.merge_item_id")"
expect_value "$merge_show_output" "item.0.subject_id" "$source_merge_task_id"
expect_value "$merge_show_output" "item.0.resolution" "none"

capture_failure "merge-freeze-unresolved" merge_freeze_unresolved \
    --error-format key-value \
    merge freeze "$store" \
    --merge "$merge_id"
expect_value "$merge_freeze_unresolved" "error_code" "workspace_invalid"
expect_value "$merge_freeze_unresolved" "error_category" "workspace"
expect_value "$merge_freeze_unresolved" "retryable" "false"
expect_contains "$merge_freeze_unresolved" "unresolved item"

capture_success "merge-resolve-recovery" merge_resolve_output \
    merge resolve "$store" \
    --item "$merge_item_id" \
    --kind theirs \
    --session "$claim_taking_session_id" \
    --rationale-json '{"reason":"operator recovery matrix accepts source item"}' \
    --expected-merge "$merge_id" \
    --expected-resolution theirs \
    --expected-resolved-by-session "$claim_taking_session_id"
expect_value "$merge_resolve_output" "resolution" "theirs"
capture_success "merge-freeze-recovery" merge_freeze_output \
    merge freeze "$store" \
    --merge "$merge_id" \
    --expected-merge "$merge_id" \
    --expected-frozen-items 1
expect_value "$merge_freeze_output" "frozen_items" "1"
capture_success "merge-continue-recovery" merge_continue_output \
    merge continue "$store" \
    --merge "$merge_id" \
    --session "$claim_taking_session_id" \
    --detail-json '{"reason":"operator recovery matrix completes merge after explicit resolution"}' \
    --expected-runtime-state completed \
    --expected-target-branch "$branch_id" \
    --expected-source-branch "$source_branch_id" \
    --expected-continued-by-session "$claim_taking_session_id"
merge_result_commit_id="$(value "$merge_continue_output" "result_commit_id")"
head_commit_id="$merge_result_commit_id"
expect_value "$merge_continue_output" "runtime_state" "completed"

capture_success "final-integrity" final_integrity_output \
    store integrity "$store" \
    --require-valid \
    --expected-checked-branches 2
expect_value "$final_integrity_output" "valid_required" "true"

summary="$(
    printf 'phase4ni_operator_recovery_maturity=PASS\n'
    printf 'log_dir=%s\n' "$log_dir"
    printf 'tmp_dir=%s\n' "$tmp_dir"
    printf 'store=%s\n' "$store"
    printf 'core_error_codes=%s\n' "$core_error_codes"
    printf 'guide_error_codes=%s\n' "$guide_error_codes"
    printf 'guide_coverage_missing=0\n'
    printf 'guide_coverage_extra=0\n'
    printf 'guide_retryability_matches_core_rule=true\n'
    printf 'cli_parse_error_json_recovery=passed\n'
    printf 'branch_head_conflict_error_code=%s\n' "$(value "$branch_conflict_output" "error_code")"
    printf 'branch_head_conflict_retryable=%s\n' "$(value "$branch_conflict_output" "retryable")"
    printf 'branch_head_conflict_recovery=passed\n'
    printf 'resource_drift_reason_code=%s\n' "$(value "$resource_drift_output" "reason_code")"
    printf 'resource_drift_recovery=applicable\n'
    printf 'resource_unavailable_reason_code=%s\n' "$(value "$resource_unavailable_output" "reason_code")"
    printf 'resource_unavailable_recovery=applicable\n'
    printf 'resource_error_reason_code=%s\n' "$(value "$resource_error_output" "reason_code")"
    printf 'resource_error_recovery=applicable\n'
    printf 'resource_final_ac_status=%s\n' "$(value "$resource_final_ac_status" "status")"
    printf 'claim_guard_reason=%s\n' "$(value "$claim_blocked_guard_output" "reason")"
    printf 'claim_takeover_precondition_error_code=%s\n' "$(value "$claim_takeover_before_stale" "error_code")"
    printf 'claim_takeover_recovery_claim_id=%s\n' "$claim_takeover_id"
    printf 'claim_takeover_recovery=passed\n'
    printf 'merge_unresolved_error_code=%s\n' "$(value "$merge_freeze_unresolved" "error_code")"
    printf 'merge_unresolved_recovery=completed\n'
    printf 'merge_result_commit_id=%s\n' "$merge_result_commit_id"
    printf 'final_integrity_valid_required=%s\n' "$(value "$final_integrity_output" "valid_required")"
)"
write_log "summary" "$summary"
printf '%s\n' "$summary"
