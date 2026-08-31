#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"

task_count="${WORKVCS_LARGER_STORE_TASKS:-48}"
baseline_tasks="${WORKVCS_LARGER_STORE_BASELINE_TASKS:-$((task_count / 2))}"
verification_count="${WORKVCS_LARGER_STORE_VERIFICATIONS:-8}"
relation_pair_count="${WORKVCS_LARGER_STORE_RELATION_PAIRS:-16}"

output_root="${WORKVCS_LARGER_STORE_OUTPUT_ROOT:-}"
keep_tmp="${WORKVCS_LARGER_STORE_KEEP_TMP:-0}"
if [[ -n "$output_root" ]]; then
    mkdir -p "$output_root"
    run_dir="$(mktemp -d "$output_root/larger-store-portability.XXXXXX")"
    keep_tmp=1
else
    run_dir="$(mktemp -d "${TMPDIR:-/tmp}/workvcs-larger-store-portability.XXXXXX")"
fi
log_file="$run_dir/run.log"

cleanup() {
    local status=$?
    if [[ "$status" -ne 0 ]]; then
        printf 'larger_store_error=failed\n' >&2
        printf 'larger_store_log=%s\n' "$log_file" >&2
        if [[ -f "$log_file" ]]; then
            printf 'larger_store_log_tail_begin\n' >&2
            tail -n 80 "$log_file" >&2 || true
            printf 'larger_store_log_tail_end\n' >&2
        fi
    elif [[ "$keep_tmp" != "1" ]]; then
        rm -rf "$run_dir"
    fi
}
trap cleanup EXIT

die() {
    printf 'larger_store_error=%s\n' "$*" >&2
    exit 1
}

step() {
    printf 'larger_store_step=%s\n' "$*" >&2
    printf 'step=%s\n' "$*" >>"$log_file"
}

require_positive_integer() {
    local name="$1"
    local value="$2"
    [[ "$value" =~ ^[0-9]+$ && "$value" -gt 0 ]] || die "${name} must be a positive integer"
}

require_positive_integer "WORKVCS_LARGER_STORE_TASKS" "$task_count"
require_positive_integer "WORKVCS_LARGER_STORE_BASELINE_TASKS" "$baseline_tasks"
require_positive_integer "WORKVCS_LARGER_STORE_VERIFICATIONS" "$verification_count"
require_positive_integer "WORKVCS_LARGER_STORE_RELATION_PAIRS" "$relation_pair_count"

(( task_count >= 4 )) || die "WORKVCS_LARGER_STORE_TASKS must be at least 4"
(( baseline_tasks < task_count )) || die "baseline task count must be lower than total task count"
(( verification_count <= task_count - baseline_tasks )) || die "verification count must fit in post-baseline tasks"
(( relation_pair_count <= task_count - baseline_tasks )) || die "relation pair count must fit in post-baseline tasks"

workvcs_bin="$repo_root/target/debug/workvcs"
source_store="$run_dir/source.sqlite"
target_store="$run_dir/target.sqlite"
bundle_dir="$run_dir/bundle"

run_build() {
    step "build cli"
    {
        printf 'cmd=(cd %q && cargo build -q -p workvcs-cli)\n' "$repo_root"
        (cd "$repo_root" && cargo build -q -p workvcs-cli)
        printf 'exit=0\n---\n'
    } >>"$log_file" 2>&1 || die "cargo build failed"
    [[ -x "$workvcs_bin" ]] || die "missing built CLI at $workvcs_bin"
}

run_workvcs() {
    local output
    local status
    {
        printf 'cmd=workvcs'
        printf ' %q' "$@"
        printf '\n'
    } >>"$log_file"
    set +e
    output="$("$workvcs_bin" "$@" 2>&1)"
    status=$?
    set -e
    {
        printf 'exit=%s\n' "$status"
        printf '%s\n' "$output"
        printf '%s\n' '---'
    } >>"$log_file"
    if [[ "$status" -ne 0 ]]; then
        printf '%s\n' "$output" >&2
        die "workvcs command failed"
    fi
    printf '%s\n' "$output"
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

expect_int_ge() {
    local output="$1"
    local key="$2"
    local minimum="$3"
    local actual
    actual="$(value "$output" "$key")"
    [[ "$actual" =~ ^[0-9]+$ ]] || die "${key} is not an integer: ${actual}"
    (( actual >= minimum )) || die "${key} expected >= ${minimum} got ${actual}"
}

expect_integrity_valid() {
    local output="$1"
    expect_value "$output" "valid_required" "true"
    expect_value "$output" "invalid_checkpoints_match_expected" "true"
}

run_started_at="$SECONDS"

run_build

step "initialize source workspace"
init_output="$(run_workvcs init "$source_store" --display-name "larger-store-portability-source")"
expect_contains "$init_output" "initialized store_id="
store_info_output="$(run_workvcs store info "$source_store")"
store_id="$(value "$store_info_output" "store_id")"

workspace_output="$(run_workvcs workspace create "$source_store" --display-name "larger-store-portability")"
workspace_id="$(value "$workspace_output" "workspace_id")"
branch_id="$(value "$workspace_output" "branch_id")"
head_commit_id="$(value "$workspace_output" "genesis_commit_id")"
expect_value "$workspace_output" "branch_name" "main"

declare -a task_ids
declare -a task_version_ids
declare -a criterion_ids
declare -a requirement_ids
declare -a verification_ids

step "create baseline tasks"
for ((i = 1; i <= task_count; i += 1)); do
    task_output="$(run_workvcs \
        task create "$source_store" \
        --branch "$branch_id" \
        --head "$head_commit_id" \
        --description "Larger Store portability task ${i}" \
        --priority "$(((i % 9) + 1))")"
    task_ids[$i]="$(value "$task_output" "task_entity_id")"
    task_version_ids[$i]="$(value "$task_output" "task_entity_version_id")"
    head_commit_id="$(value "$task_output" "commit_id")"
    expect_value "$task_output" "status" "pending"

    if (( i == baseline_tasks )); then
        step "copy baseline target store"
        cp "$source_store" "$target_store"
    fi
done

step "create acceptance criteria and verification requirements"
for ((i = 1; i <= verification_count; i += 1)); do
    task_index=$((baseline_tasks + i))
    criterion_output="$(run_workvcs \
        ac create "$source_store" \
        --branch "$branch_id" \
        --head "$head_commit_id" \
        --task "${task_ids[$task_index]}" \
        --task-version "${task_version_ids[$task_index]}" \
        --local-key "AC-LARGE-${i}" \
        --statement "Larger Store portability criterion ${i} is satisfied.")"
    criterion_ids[$i]="$(value "$criterion_output" "acceptance_criterion_entity_id")"
    criterion_version_id="$(value "$criterion_output" "acceptance_criterion_entity_version_id")"
    task_version_ids[$task_index]="$(value "$criterion_output" "task_entity_version_id")"
    head_commit_id="$(value "$criterion_output" "commit_id")"
    expect_value "$criterion_output" "classification" "required"

    requirement_output="$(run_workvcs \
        vr create "$source_store" \
        --branch "$branch_id" \
        --head "$head_commit_id" \
        --criterion "${criterion_ids[$i]}" \
        --criterion-version "$criterion_version_id" \
        --local-key "VR-LARGE-${i}" \
        --statement "Record larger Store portability verification ${i}.")"
    requirement_ids[$i]="$(value "$requirement_output" "verification_requirement_entity_id")"
    head_commit_id="$(value "$requirement_output" "commit_id")"
    expect_value "$requirement_output" "local_key" "VR-LARGE-${i}"

    printf -v metadata_json '{"phase":"4LQ","index":%d}' "$i"
    verification_output="$(run_workvcs \
        verify "$source_store" \
        --branch "$branch_id" \
        --head "$head_commit_id" \
        --verification-requirement "${requirement_ids[$i]}" \
        --result passed \
        --method manual-review \
        --evidence-kind manual-review \
        --evidence-metadata-json "$metadata_json" \
        --evidence-content-role log \
        --evidence-content "Larger Store portability verification evidence ${i}" \
        --evidence-media-type text/plain \
        --expected-branch "$branch_id" \
        --expected-head "$head_commit_id" \
        --expected-target-kind verification_requirement \
        --expected-target "${requirement_ids[$i]}" \
        --expected-result passed \
        --expected-evidence-kind manual-review \
        --expected-evidence-relations 1)"
    verification_ids[$i]="$(value "$verification_output" "verification_entity_id")"
    head_commit_id="$(value "$verification_output" "commit_id")"
    expect_value "$verification_output" "target_kind_match_expected" "true"
    expect_value "$verification_output" "result_match_expected" "true"
    expect_value "$verification_output" "evidence_kind_match_expected" "true"
    expect_value "$verification_output" "evidence_relations_match_expected" "true"
done

step "create scheduling relations"
for ((i = 1; i <= relation_pair_count; i += 1)); do
    later_task_index=$((baseline_tasks + i))
    prerequisite_task_index="$i"
    dependency_output="$(run_workvcs \
        task depends-on "$source_store" \
        --branch "$branch_id" \
        --head "$head_commit_id" \
        --task "${task_ids[$later_task_index]}" \
        --depends-on "${task_ids[$prerequisite_task_index]}")"
    head_commit_id="$(value "$dependency_output" "commit_id")"
    expect_value "$dependency_output" "relation_type" "depends_on"

    order_output="$(run_workvcs \
        task ordered-before "$source_store" \
        --branch "$branch_id" \
        --head "$head_commit_id" \
        --earlier "${task_ids[$prerequisite_task_index]}" \
        --later "${task_ids[$later_task_index]}")"
    head_commit_id="$(value "$order_output" "commit_id")"
    expect_value "$order_output" "relation_type" "ordered_before"
done
relation_count=$((relation_pair_count * 2))

step "checkpoint and export"
source_head_output="$(run_workvcs \
    branch head "$source_store" \
    --branch "$branch_id")"
source_state_digest="$(value "$source_head_output" "state_digest")"
expect_value "$source_head_output" "head_commit_id" "$head_commit_id"

checkpoint_output="$(run_workvcs checkpoint create "$source_store" --commit "$head_commit_id")"
checkpoint_id="$(value "$checkpoint_output" "checkpoint_id")"
checkpoint_content_digest="$(value "$checkpoint_output" "content_digest")"
expect_value "$checkpoint_output" "state_digest" "$source_state_digest"
expect_value "$checkpoint_output" "usability_state" "usable"

bundle_summary_output="$(run_workvcs \
    bundle export "$source_store" \
    --commit "$head_commit_id" \
    --expected-state-digest "$source_state_digest")"
expect_value "$bundle_summary_output" "state_matches_expected" "true"
expect_int_ge "$bundle_summary_output" "relations" "$relation_count"
expect_int_ge "$bundle_summary_output" "relation_versions" "$relation_count"
expect_value "$bundle_summary_output" "checkpoint_candidates" "1"

bundle_export_dir_output="$(run_workvcs \
    bundle export-dir "$source_store" \
    --commit "$head_commit_id" \
    --output-dir "$bundle_dir")"
payload_files="$(value "$bundle_export_dir_output" "payload_files")"
payload_references="$(value "$bundle_export_dir_output" "payload_references")"
expect_int_ge "$bundle_export_dir_output" "payload_files" 1
expect_int_ge "$bundle_export_dir_output" "payload_references" 1

bundle_validate_output="$(run_workvcs \
    bundle validate-dir "$source_store" \
    --commit "$head_commit_id" \
    --input-dir "$bundle_dir" \
    --require-valid)"
expect_value "$bundle_validate_output" "valid" "true"
expect_value "$bundle_validate_output" "problem" "none"

step "preflight and apply target"
bundle_preflight_output="$(run_workvcs \
    bundle preflight-dir "$target_store" \
    --input-dir "$bundle_dir" \
    --require-valid \
    --require-can-apply \
    --expected-exported-branch-heads 1 \
    --expected-branch-heads-already-present 0 \
    --expected-branch-heads-missing 0 \
    --expected-branch-heads-fast-forward 1 \
    --expected-branch-heads-diverged 0)"
expect_value "$bundle_preflight_output" "valid" "true"
expect_value "$bundle_preflight_output" "can_apply" "true"
expect_value "$bundle_preflight_output" "action" "same_store_fast_forward_ready"
expect_value "$bundle_preflight_output" "source_store_relation" "same_store"
expect_value "$bundle_preflight_output" "branch_heads_fast_forward_match_expected" "true"

bundle_apply_output="$(run_workvcs \
    bundle apply-dir "$target_store" \
    --input-dir "$bundle_dir" \
    --require-applied \
    --expected-outcome same_store_fast_forward_applied \
    --expected-updated-branch-heads 1)"
bundle_import_id="$(value "$bundle_apply_output" "import_id")"
bundle_digest="$(value "$bundle_apply_output" "bundle_digest")"
expect_value "$bundle_apply_output" "applied" "true"
expect_value "$bundle_apply_output" "outcome" "same_store_fast_forward_applied"
expect_value "$bundle_apply_output" "updated_branch_heads_match_expected" "true"
expect_int_ge "$bundle_apply_output" "imported_commits" "$((task_count - baseline_tasks + verification_count * 3 + relation_count))"
expect_int_ge "$bundle_apply_output" "imported_entity_versions" "$((task_count - baseline_tasks + verification_count * 5))"
expect_int_ge "$bundle_apply_output" "imported_relation_versions" "$relation_count"
expect_int_ge "$bundle_apply_output" "imported_evidences" "$verification_count"
expect_int_ge "$bundle_apply_output" "imported_content_objects" "$verification_count"
expect_int_ge "$bundle_apply_output" "imported_verification_bases" "$verification_count"
expect_value "$bundle_apply_output" "imported_checkpoints" "1"
expect_value "$bundle_apply_output" "imported_checkpoint_statuses" "1"

step "validate target state"
target_head_output="$(run_workvcs \
    branch head "$target_store" \
    --branch "$branch_id" \
    --expected-state-digest "$source_state_digest")"
expect_value "$target_head_output" "head_commit_id" "$head_commit_id"
expect_value "$target_head_output" "matches_expected" "true"

target_task_list_output="$(run_workvcs \
    task list "$target_store" \
    --branch "$branch_id" \
    --expected-tasks "$task_count")"
expect_value "$target_task_list_output" "tasks" "$task_count"
expect_value "$target_task_list_output" "tasks_match_expected" "true"

target_scheduling_output="$(run_workvcs \
    task scheduling-list "$target_store" \
    --branch "$branch_id" \
    --expected-relations "$relation_count")"
expect_value "$target_scheduling_output" "relations" "$relation_count"
expect_value "$target_scheduling_output" "relations_match_expected" "true"

target_requirement_list_output="$(run_workvcs \
    vr list "$target_store" \
    --branch "$branch_id" \
    --expected-requirements "$verification_count")"
expect_value "$target_requirement_list_output" "requirements" "$verification_count"
expect_value "$target_requirement_list_output" "requirements_match_expected" "true"

target_verification_list_output="$(run_workvcs \
    verification list "$target_store" \
    --branch "$branch_id" \
    --target-kind verification_requirement \
    --expected-verifications "$verification_count")"
expect_value "$target_verification_list_output" "verifications" "$verification_count"
expect_value "$target_verification_list_output" "verifications_match_expected" "true"

target_ac_status_output="$(run_workvcs \
    ac status "$target_store" \
    --branch "$branch_id" \
    --criterion "${criterion_ids[1]}")"
expect_value "$target_ac_status_output" "status" "verified"

target_checkpoint_latest_output="$(run_workvcs \
    checkpoint latest "$target_store" \
    --commit "$head_commit_id" \
    --require-found \
    --expected-checkpoint "$checkpoint_id")"
expect_value "$target_checkpoint_latest_output" "checkpoint_found" "true"
expect_value "$target_checkpoint_latest_output" "checkpoint_matches_expected" "true"

target_checkpoint_show_output="$(run_workvcs \
    checkpoint show "$target_store" \
    --checkpoint "$checkpoint_id" \
    --expected-state-digest "$source_state_digest" \
    --expected-content-digest "$checkpoint_content_digest")"
expect_value "$target_checkpoint_show_output" "state_matches_expected" "true"
expect_value "$target_checkpoint_show_output" "content_matches_expected" "true"

target_checkpoint_validate_output="$(run_workvcs \
    checkpoint validate "$target_store" \
    --checkpoint "$checkpoint_id" \
    --require-valid)"
expect_value "$target_checkpoint_validate_output" "valid" "true"
expect_value "$target_checkpoint_validate_output" "problem" "none"

target_import_show_output="$(run_workvcs \
    bundle import-show "$target_store" \
    --import "$bundle_import_id" \
    --expected-bundle-digest "$bundle_digest" \
    --expected-outcome same_store_fast_forward_applied)"
expect_value "$target_import_show_output" "bundle_matches_expected" "true"
expect_value "$target_import_show_output" "outcome_matches_expected" "true"

step "restore target after local work"
target_local_task_output="$(run_workvcs \
    task create "$target_store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --description "Target-local work after bundle apply" \
    --priority 9)"
target_local_head_commit_id="$(value "$target_local_task_output" "commit_id")"
expect_value "$target_local_task_output" "status" "pending"

restore_output="$(run_workvcs \
    restore "$target_store" \
    --branch "$branch_id" \
    --head "$target_local_head_commit_id" \
    --target-commit "$head_commit_id" \
    --rationale-json '{"reason":"larger Store portability rollback to bundle head"}')"
restore_commit_id="$(value "$restore_output" "commit_id")"
expect_value "$restore_output" "previous_head_commit_id" "$target_local_head_commit_id"
expect_value "$restore_output" "target_commit_id" "$head_commit_id"
expect_value "$restore_output" "work_state_digest" "$source_state_digest"
expect_int_ge "$restore_output" "operation_count" 1

restored_show_output="$(run_workvcs \
    show-at "$target_store" \
    --commit "$restore_commit_id" \
    --expected-state-digest "$source_state_digest")"
expect_value "$restored_show_output" "matches_expected" "true"

restored_task_list_output="$(run_workvcs \
    task list "$target_store" \
    --commit "$restore_commit_id" \
    --expected-tasks "$task_count")"
expect_value "$restored_task_list_output" "tasks" "$task_count"
expect_value "$restored_task_list_output" "tasks_match_expected" "true"

step "integrity"
source_integrity_output="$(run_workvcs \
    store integrity "$source_store" \
    --require-valid \
    --expected-invalid-checkpoints 0)"
expect_integrity_valid "$source_integrity_output"

target_integrity_output="$(run_workvcs \
    store integrity "$target_store" \
    --require-valid \
    --expected-invalid-checkpoints 0)"
expect_integrity_valid "$target_integrity_output"

doctor_output="$(run_workvcs \
    doctor "$target_store" \
    --require-valid \
    --expected-invalid-checkpoints 0)"
expect_integrity_valid "$doctor_output"

elapsed_seconds=$((SECONDS - run_started_at))
printf 'larger_store_result=PASS\n'
printf 'store_id=%s\n' "$store_id"
printf 'workspace_id=%s\n' "$workspace_id"
printf 'branch_id=%s\n' "$branch_id"
printf 'task_count=%s\n' "$task_count"
printf 'baseline_tasks=%s\n' "$baseline_tasks"
printf 'verification_requirements=%s\n' "$verification_count"
printf 'verification_records=%s\n' "$verification_count"
printf 'relation_pairs=%s\n' "$relation_pair_count"
printf 'relation_versions=%s\n' "$relation_count"
printf 'source_head_commit_id=%s\n' "$head_commit_id"
printf 'source_state_digest=%s\n' "$source_state_digest"
printf 'checkpoint_id=%s\n' "$checkpoint_id"
printf 'bundle_import_id=%s\n' "$bundle_import_id"
printf 'bundle_digest=%s\n' "$bundle_digest"
printf 'payload_files=%s\n' "$payload_files"
printf 'payload_references=%s\n' "$payload_references"
printf 'imported_commits=%s\n' "$(value "$bundle_apply_output" "imported_commits")"
printf 'imported_entity_versions=%s\n' "$(value "$bundle_apply_output" "imported_entity_versions")"
printf 'imported_relation_versions=%s\n' "$(value "$bundle_apply_output" "imported_relation_versions")"
printf 'imported_evidences=%s\n' "$(value "$bundle_apply_output" "imported_evidences")"
printf 'imported_content_objects=%s\n' "$(value "$bundle_apply_output" "imported_content_objects")"
printf 'imported_verification_bases=%s\n' "$(value "$bundle_apply_output" "imported_verification_bases")"
printf 'updated_branch_heads=%s\n' "$(value "$bundle_apply_output" "updated_branch_heads")"
printf 'restore_commit_id=%s\n' "$restore_commit_id"
printf 'elapsed_seconds=%s\n' "$elapsed_seconds"
if [[ "$keep_tmp" == "1" ]]; then
    printf 'output_dir=%s\n' "$run_dir"
    printf 'log_file=%s\n' "$log_file"
else
    printf 'output_dir=removed_on_success\n'
    printf 'log_file=removed_on_success\n'
fi
