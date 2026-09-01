#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"

cycle_count="${WORKVCS_MAINTAINED_STORE_CYCLES:-3}"
seed_tasks="${WORKVCS_MAINTAINED_STORE_SEED_TASKS:-4}"
tasks_per_cycle="${WORKVCS_MAINTAINED_STORE_TASKS_PER_CYCLE:-6}"
verifications_per_cycle="${WORKVCS_MAINTAINED_STORE_VERIFICATIONS_PER_CYCLE:-1}"
relation_pairs_per_cycle="${WORKVCS_MAINTAINED_STORE_RELATION_PAIRS_PER_CYCLE:-2}"

output_root="${WORKVCS_MAINTAINED_STORE_OUTPUT_ROOT:-}"
keep_tmp="${WORKVCS_MAINTAINED_STORE_KEEP_TMP:-0}"
if [[ -n "$output_root" ]]; then
    mkdir -p "$output_root"
    run_dir="$(mktemp -d "$output_root/maintained-store-portability.XXXXXX")"
    keep_tmp=1
else
    run_dir="$(mktemp -d "${TMPDIR:-/tmp}/workvcs-maintained-store-portability.XXXXXX")"
fi
log_file="$run_dir/run.log"

cleanup() {
    local status=$?
    if [[ "$status" -ne 0 ]]; then
        printf 'maintained_store_error=failed\n' >&2
        printf 'maintained_store_log=%s\n' "$log_file" >&2
        if [[ -f "$log_file" ]]; then
            printf 'maintained_store_log_tail_begin\n' >&2
            tail -n 80 "$log_file" >&2 || true
            printf 'maintained_store_log_tail_end\n' >&2
        fi
    elif [[ "$keep_tmp" != "1" ]]; then
        rm -rf "$run_dir"
    fi
}
trap cleanup EXIT

die() {
    printf 'maintained_store_error=%s\n' "$*" >&2
    exit 1
}

step() {
    printf 'maintained_store_step=%s\n' "$*" >&2
    printf 'step=%s\n' "$*" >>"$log_file"
}

require_positive_integer() {
    local name="$1"
    local value="$2"
    [[ "$value" =~ ^[0-9]+$ && "$value" -gt 0 ]] || die "${name} must be a positive integer"
}

require_positive_integer "WORKVCS_MAINTAINED_STORE_CYCLES" "$cycle_count"
require_positive_integer "WORKVCS_MAINTAINED_STORE_SEED_TASKS" "$seed_tasks"
require_positive_integer "WORKVCS_MAINTAINED_STORE_TASKS_PER_CYCLE" "$tasks_per_cycle"
require_positive_integer "WORKVCS_MAINTAINED_STORE_VERIFICATIONS_PER_CYCLE" "$verifications_per_cycle"
require_positive_integer "WORKVCS_MAINTAINED_STORE_RELATION_PAIRS_PER_CYCLE" "$relation_pairs_per_cycle"

(( cycle_count >= 2 )) || die "WORKVCS_MAINTAINED_STORE_CYCLES must be at least 2"
(( seed_tasks >= 1 )) || die "WORKVCS_MAINTAINED_STORE_SEED_TASKS must be at least 1"
(( tasks_per_cycle >= 2 )) || die "WORKVCS_MAINTAINED_STORE_TASKS_PER_CYCLE must be at least 2"
(( verifications_per_cycle <= tasks_per_cycle )) || die "verification count must fit in cycle tasks"
(( relation_pairs_per_cycle <= tasks_per_cycle )) || die "relation pair count must fit in cycle tasks"

workvcs_bin="$repo_root/target/debug/workvcs"
source_store="$run_dir/source.sqlite"
target_store="$run_dir/target.sqlite"
bundle_root="$run_dir/bundles"

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

declare -a task_ids
declare -a task_version_ids
declare -a criterion_ids
declare -a requirement_ids
declare -a verification_ids
declare -a cycle_head_commit_ids
declare -a cycle_state_digests
declare -a cycle_checkpoint_ids
declare -a cycle_lineage_counts
declare -a cycle_bundle_import_ids
declare -a cycle_bundle_digests
declare -a cycle_payload_files
declare -a cycle_payload_references
declare -a cycle_restore_commit_ids

run_started_at="$SECONDS"

run_build

step "initialize source workspace"
init_output="$(run_workvcs init "$source_store" --display-name "maintained-store-portability-source")"
expect_contains "$init_output" "initialized store_id="
store_info_output="$(run_workvcs store info "$source_store")"
store_id="$(value "$store_info_output" "store_id")"

workspace_output="$(run_workvcs workspace create "$source_store" --display-name "maintained-store-portability")"
workspace_id="$(value "$workspace_output" "workspace_id")"
branch_id="$(value "$workspace_output" "branch_id")"
head_commit_id="$(value "$workspace_output" "genesis_commit_id")"
expect_value "$workspace_output" "branch_name" "main"

total_tasks=0
total_requirements=0
total_verifications=0
total_relation_versions=0

step "create seed tasks"
for ((i = 1; i <= seed_tasks; i += 1)); do
    task_output="$(run_workvcs \
        task create "$source_store" \
        --branch "$branch_id" \
        --head "$head_commit_id" \
        --description "Maintained Store seed task ${i}" \
        --priority "$(((i % 9) + 1))")"
    total_tasks=$((total_tasks + 1))
    task_ids[$total_tasks]="$(value "$task_output" "task_entity_id")"
    task_version_ids[$total_tasks]="$(value "$task_output" "task_entity_version_id")"
    head_commit_id="$(value "$task_output" "commit_id")"
    expect_value "$task_output" "status" "pending"
done

baseline_head_output="$(run_workvcs branch head "$source_store" --branch "$branch_id")"
source_state_digest="$(value "$baseline_head_output" "state_digest")"
expect_value "$baseline_head_output" "head_commit_id" "$head_commit_id"
baseline_head_commit_id="$head_commit_id"
baseline_state_digest="$source_state_digest"

step "copy baseline target store"
cp "$source_store" "$target_store"
target_main_head_commit_id="$baseline_head_commit_id"
target_main_state_digest="$baseline_state_digest"

baseline_source_integrity="$(run_workvcs store integrity "$source_store" --require-valid --expected-invalid-checkpoints 0)"
expect_integrity_valid "$baseline_source_integrity"
baseline_target_integrity="$(run_workvcs store integrity "$target_store" --require-valid --expected-invalid-checkpoints 0)"
expect_integrity_valid "$baseline_target_integrity"

target_restore_checks=0
lineage_list_checks=0
mkdir -p "$bundle_root"

for ((cycle = 1; cycle <= cycle_count; cycle += 1)); do
    step "cycle ${cycle} reopen source"
    source_reopen_info="$(run_workvcs store info "$source_store")"
    expect_value "$source_reopen_info" "store_id" "$store_id"
    source_reopen_head="$(run_workvcs \
        branch head "$source_store" \
        --branch "$branch_id" \
        --expected-state-digest "$source_state_digest")"
    expect_value "$source_reopen_head" "head_commit_id" "$head_commit_id"
    expect_value "$source_reopen_head" "matches_expected" "true"

    step "cycle ${cycle} source maintenance"
    first_cycle_task_index=$((total_tasks + 1))
    for ((i = 1; i <= tasks_per_cycle; i += 1)); do
        global_task_number=$((total_tasks + 1))
        task_output="$(run_workvcs \
            task create "$source_store" \
            --branch "$branch_id" \
            --head "$head_commit_id" \
            --description "Maintained Store cycle ${cycle} task ${i}" \
            --priority "$((((cycle + i) % 9) + 1))")"
        total_tasks=$((total_tasks + 1))
        task_ids[$global_task_number]="$(value "$task_output" "task_entity_id")"
        task_version_ids[$global_task_number]="$(value "$task_output" "task_entity_version_id")"
        head_commit_id="$(value "$task_output" "commit_id")"
        expect_value "$task_output" "status" "pending"
    done

    for ((i = 1; i <= verifications_per_cycle; i += 1)); do
        task_index=$((first_cycle_task_index + i - 1))
        criterion_output="$(run_workvcs \
            ac create "$source_store" \
            --branch "$branch_id" \
            --head "$head_commit_id" \
            --task "${task_ids[$task_index]}" \
            --task-version "${task_version_ids[$task_index]}" \
            --local-key "AC-MAINTAINED-${cycle}-${i}" \
            --statement "Maintained Store cycle ${cycle} criterion ${i} is satisfied.")"
        total_requirements=$((total_requirements + 1))
        criterion_ids[$total_requirements]="$(value "$criterion_output" "acceptance_criterion_entity_id")"
        criterion_version_id="$(value "$criterion_output" "acceptance_criterion_entity_version_id")"
        task_version_ids[$task_index]="$(value "$criterion_output" "task_entity_version_id")"
        head_commit_id="$(value "$criterion_output" "commit_id")"
        expect_value "$criterion_output" "classification" "required"

        requirement_output="$(run_workvcs \
            vr create "$source_store" \
            --branch "$branch_id" \
            --head "$head_commit_id" \
            --criterion "${criterion_ids[$total_requirements]}" \
            --criterion-version "$criterion_version_id" \
            --local-key "VR-MAINTAINED-${cycle}-${i}" \
            --statement "Record maintained Store portability verification ${cycle}.${i}.")"
        requirement_ids[$total_requirements]="$(value "$requirement_output" "verification_requirement_entity_id")"
        head_commit_id="$(value "$requirement_output" "commit_id")"
        expect_value "$requirement_output" "local_key" "VR-MAINTAINED-${cycle}-${i}"

        printf -v metadata_json '{"phase":"4ND","cycle":%d,"index":%d}' "$cycle" "$i"
        verification_output="$(run_workvcs \
            verify "$source_store" \
            --branch "$branch_id" \
            --head "$head_commit_id" \
            --verification-requirement "${requirement_ids[$total_requirements]}" \
            --result passed \
            --method manual-review \
            --evidence-kind manual-review \
            --evidence-metadata-json "$metadata_json" \
            --evidence-content-role log \
            --evidence-content "Maintained Store portability verification evidence ${cycle}.${i}" \
            --evidence-media-type text/plain \
            --expected-branch "$branch_id" \
            --expected-head "$head_commit_id" \
            --expected-target-kind verification_requirement \
            --expected-target "${requirement_ids[$total_requirements]}" \
            --expected-result passed \
            --expected-evidence-kind manual-review \
            --expected-evidence-relations 1)"
        verification_ids[$total_requirements]="$(value "$verification_output" "verification_entity_id")"
        total_verifications=$((total_verifications + 1))
        head_commit_id="$(value "$verification_output" "commit_id")"
        expect_value "$verification_output" "target_kind_match_expected" "true"
        expect_value "$verification_output" "result_match_expected" "true"
        expect_value "$verification_output" "evidence_kind_match_expected" "true"
        expect_value "$verification_output" "evidence_relations_match_expected" "true"
    done

    prior_task_count=$((first_cycle_task_index - 1))
    for ((i = 1; i <= relation_pairs_per_cycle; i += 1)); do
        later_task_index=$((first_cycle_task_index + i - 1))
        prerequisite_task_index=$((1 + ((i - 1) % prior_task_count)))
        dependency_output="$(run_workvcs \
            task depends-on "$source_store" \
            --branch "$branch_id" \
            --head "$head_commit_id" \
            --task "${task_ids[$later_task_index]}" \
            --depends-on "${task_ids[$prerequisite_task_index]}")"
        head_commit_id="$(value "$dependency_output" "commit_id")"
        expect_value "$dependency_output" "relation_type" "depends_on"
        total_relation_versions=$((total_relation_versions + 1))

        order_output="$(run_workvcs \
            task ordered-before "$source_store" \
            --branch "$branch_id" \
            --head "$head_commit_id" \
            --earlier "${task_ids[$prerequisite_task_index]}" \
            --later "${task_ids[$later_task_index]}")"
        head_commit_id="$(value "$order_output" "commit_id")"
        expect_value "$order_output" "relation_type" "ordered_before"
        total_relation_versions=$((total_relation_versions + 1))
    done

    step "cycle ${cycle} checkpoint and export"
    source_head_output="$(run_workvcs branch head "$source_store" --branch "$branch_id")"
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
    expect_int_ge "$bundle_summary_output" "checkpoint_candidates" 1

    bundle_dir="$bundle_root/cycle-${cycle}"
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

    step "cycle ${cycle} preflight and apply target"
    target_reopen_info="$(run_workvcs store info "$target_store")"
    expect_value "$target_reopen_info" "store_id" "$store_id"
    target_head_before_output="$(run_workvcs \
        branch head "$target_store" \
        --branch "$branch_id" \
        --expected-state-digest "$target_main_state_digest")"
    expect_value "$target_head_before_output" "head_commit_id" "$target_main_head_commit_id"
    expect_value "$target_head_before_output" "matches_expected" "true"

    bundle_preflight_output="$(run_workvcs \
        bundle preflight-dir "$target_store" \
        --input-dir "$bundle_dir" \
        --require-valid \
        --require-can-apply \
        --expected-exported-branch-heads 1 \
        --expected-branch-heads-already-present 0 \
        --expected-branch-heads-missing 0 \
        --expected-branch-heads-fast-forward 1 \
        --expected-branch-heads-diverged 0 \
        --expected-branch-head-detail-count 1 \
        --expected-first-branch-head-status fast_forward \
        --expected-first-branch-head-source-head "$head_commit_id" \
        --expected-first-branch-head-target-head "$target_main_head_commit_id" \
        --expected-first-branch-head-merge-base "$target_main_head_commit_id")"
    expect_value "$bundle_preflight_output" "valid" "true"
    expect_value "$bundle_preflight_output" "can_apply" "true"
    expect_value "$bundle_preflight_output" "action" "same_store_fast_forward_ready"
    expect_value "$bundle_preflight_output" "source_store_relation" "same_store"
    expect_value "$bundle_preflight_output" "branch_heads_fast_forward_match_expected" "true"
    expect_value "$bundle_preflight_output" "branch_head_detail.0.status" "fast_forward"
    expect_value "$bundle_preflight_output" "first_branch_head_source_head_match_expected" "true"
    expect_value "$bundle_preflight_output" "first_branch_head_target_head_match_expected" "true"
    expect_value "$bundle_preflight_output" "first_branch_head_merge_base_match_expected" "true"

    bundle_apply_output="$(run_workvcs \
        bundle apply-dir "$target_store" \
        --input-dir "$bundle_dir" \
        --require-applied \
        --expected-outcome same_store_fast_forward_applied \
        --expected-imported-checkpoints 1 \
        --expected-imported-checkpoint-statuses 1 \
        --expected-updated-branch-heads 1)"
    bundle_import_id="$(value "$bundle_apply_output" "import_id")"
    bundle_digest="$(value "$bundle_apply_output" "bundle_digest")"
    expect_value "$bundle_apply_output" "applied" "true"
    expect_value "$bundle_apply_output" "outcome" "same_store_fast_forward_applied"
    expect_value "$bundle_apply_output" "updated_branch_heads_match_expected" "true"
    expect_int_ge "$bundle_apply_output" "imported_commits" "$((tasks_per_cycle + verifications_per_cycle * 3 + relation_pairs_per_cycle * 2))"
    expect_int_ge "$bundle_apply_output" "imported_entity_versions" "$((tasks_per_cycle + verifications_per_cycle * 5))"
    expect_int_ge "$bundle_apply_output" "imported_relation_versions" "$((relation_pairs_per_cycle * 2))"
    expect_int_ge "$bundle_apply_output" "imported_evidences" "$verifications_per_cycle"
    expect_int_ge "$bundle_apply_output" "imported_content_objects" "$verifications_per_cycle"
    expect_int_ge "$bundle_apply_output" "imported_verification_bases" "$verifications_per_cycle"
    expect_value "$bundle_apply_output" "imported_checkpoints" "1"
    expect_value "$bundle_apply_output" "imported_checkpoint_statuses" "1"

    step "cycle ${cycle} validate target convergence"
    target_head_output="$(run_workvcs \
        branch head "$target_store" \
        --branch "$branch_id" \
        --expected-state-digest "$source_state_digest")"
    expect_value "$target_head_output" "head_commit_id" "$head_commit_id"
    expect_value "$target_head_output" "matches_expected" "true"

    target_task_list_output="$(run_workvcs \
        task list "$target_store" \
        --branch "$branch_id" \
        --expected-tasks "$total_tasks")"
    expect_value "$target_task_list_output" "tasks" "$total_tasks"
    expect_value "$target_task_list_output" "tasks_match_expected" "true"

    target_scheduling_output="$(run_workvcs \
        task scheduling-list "$target_store" \
        --branch "$branch_id" \
        --expected-relations "$total_relation_versions")"
    expect_value "$target_scheduling_output" "relations" "$total_relation_versions"
    expect_value "$target_scheduling_output" "relations_match_expected" "true"

    target_requirement_list_output="$(run_workvcs \
        vr list "$target_store" \
        --branch "$branch_id" \
        --expected-requirements "$total_requirements")"
    expect_value "$target_requirement_list_output" "requirements" "$total_requirements"
    expect_value "$target_requirement_list_output" "requirements_match_expected" "true"

    target_verification_list_output="$(run_workvcs \
        verification list "$target_store" \
        --branch "$branch_id" \
        --target-kind verification_requirement \
        --expected-verifications "$total_verifications")"
    expect_value "$target_verification_list_output" "verifications" "$total_verifications"
    expect_value "$target_verification_list_output" "verifications_match_expected" "true"

    target_ac_status_output="$(run_workvcs \
        ac status "$target_store" \
        --branch "$branch_id" \
        --criterion "${criterion_ids[$total_requirements]}")"
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

    target_lineage_list_output="$(run_workvcs \
        store lineage-list "$target_store" \
        --expected-lineages 0)"
    expect_value "$target_lineage_list_output" "lineages" "0"
    expect_value "$target_lineage_list_output" "lineages_match_expected" "true"
    lineage_list_checks=$((lineage_list_checks + 1))

    step "cycle ${cycle} integrity and doctor"
    source_integrity_output="$(run_workvcs store integrity "$source_store" --require-valid --expected-invalid-checkpoints 0)"
    expect_integrity_valid "$source_integrity_output"
    source_doctor_output="$(run_workvcs doctor "$source_store" --require-valid --expected-invalid-checkpoints 0)"
    expect_integrity_valid "$source_doctor_output"
    target_integrity_output="$(run_workvcs store integrity "$target_store" --require-valid --expected-invalid-checkpoints 0)"
    expect_integrity_valid "$target_integrity_output"
    target_doctor_output="$(run_workvcs doctor "$target_store" --require-valid --expected-invalid-checkpoints 0)"
    expect_integrity_valid "$target_doctor_output"

    if (( cycle == 1 )); then
        step "cycle ${cycle} target-local restore branch"
        target_restore_branch_output="$(run_workvcs \
            branch fork "$target_store" \
            --from-commit "$head_commit_id" \
            --name "maintained-target-restore-cycle-${cycle}")"
        target_restore_branch_id="$(value "$target_restore_branch_output" "branch_id")"
        expect_value "$target_restore_branch_output" "workspace_id" "$workspace_id"
        expect_value "$target_restore_branch_output" "head_commit_id" "$head_commit_id"

        target_local_task_output="$(run_workvcs \
            task create "$target_store" \
            --branch "$target_restore_branch_id" \
            --head "$head_commit_id" \
            --description "Target-local maintenance after maintained Store cycle ${cycle}" \
            --priority 9)"
        target_local_head_commit_id="$(value "$target_local_task_output" "commit_id")"
        expect_value "$target_local_task_output" "status" "pending"

        restore_output="$(run_workvcs \
            restore "$target_store" \
            --branch "$target_restore_branch_id" \
            --head "$target_local_head_commit_id" \
            --target-commit "$head_commit_id" \
            --rationale-json '{"reason":"maintained Store target-local rollback to imported bundle head"}')"
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
            --expected-tasks "$total_tasks")"
        expect_value "$restored_task_list_output" "tasks" "$total_tasks"
        expect_value "$restored_task_list_output" "tasks_match_expected" "true"

        target_main_after_restore_output="$(run_workvcs \
            branch head "$target_store" \
            --branch "$branch_id" \
            --expected-state-digest "$source_state_digest")"
        expect_value "$target_main_after_restore_output" "head_commit_id" "$head_commit_id"
        expect_value "$target_main_after_restore_output" "matches_expected" "true"

        cycle_restore_commit_ids[$cycle]="$restore_commit_id"
        target_restore_checks=$((target_restore_checks + 1))
    else
        cycle_restore_commit_ids[$cycle]="none"
    fi

    cycle_head_commit_ids[$cycle]="$head_commit_id"
    cycle_state_digests[$cycle]="$source_state_digest"
    cycle_checkpoint_ids[$cycle]="$checkpoint_id"
    cycle_lineage_counts[$cycle]="0"
    cycle_bundle_import_ids[$cycle]="$bundle_import_id"
    cycle_bundle_digests[$cycle]="$bundle_digest"
    cycle_payload_files[$cycle]="$payload_files"
    cycle_payload_references[$cycle]="$payload_references"
    target_main_head_commit_id="$head_commit_id"
    target_main_state_digest="$source_state_digest"
done

elapsed_seconds=$((SECONDS - run_started_at))
{
    printf 'maintained_store_result=PASS\n'
    printf 'store_id=%s\n' "$store_id"
    printf 'workspace_id=%s\n' "$workspace_id"
    printf 'branch_id=%s\n' "$branch_id"
    printf 'cycles=%s\n' "$cycle_count"
    printf 'source_reopens=%s\n' "$cycle_count"
    printf 'same_target_applies=%s\n' "$cycle_count"
    printf 'target_restore_checks=%s\n' "$target_restore_checks"
    printf 'lineage_list_checks=%s\n' "$lineage_list_checks"
    printf 'seed_tasks=%s\n' "$seed_tasks"
    printf 'tasks_per_cycle=%s\n' "$tasks_per_cycle"
    printf 'final_task_count=%s\n' "$total_tasks"
    printf 'verification_requirements=%s\n' "$total_requirements"
    printf 'verification_records=%s\n' "$total_verifications"
    printf 'relation_versions=%s\n' "$total_relation_versions"
    printf 'baseline_head_commit_id=%s\n' "$baseline_head_commit_id"
    printf 'baseline_state_digest=%s\n' "$baseline_state_digest"
    printf 'final_source_head_commit_id=%s\n' "$head_commit_id"
    printf 'final_source_state_digest=%s\n' "$source_state_digest"
    printf 'final_target_head_commit_id=%s\n' "$target_main_head_commit_id"
    printf 'final_target_state_digest=%s\n' "$target_main_state_digest"
    for ((cycle = 1; cycle <= cycle_count; cycle += 1)); do
        printf 'cycle_%s_head_commit_id=%s\n' "$cycle" "${cycle_head_commit_ids[$cycle]}"
        printf 'cycle_%s_state_digest=%s\n' "$cycle" "${cycle_state_digests[$cycle]}"
        printf 'cycle_%s_checkpoint_id=%s\n' "$cycle" "${cycle_checkpoint_ids[$cycle]}"
        printf 'cycle_%s_target_lineages=%s\n' "$cycle" "${cycle_lineage_counts[$cycle]}"
        printf 'cycle_%s_bundle_import_id=%s\n' "$cycle" "${cycle_bundle_import_ids[$cycle]}"
        printf 'cycle_%s_bundle_digest=%s\n' "$cycle" "${cycle_bundle_digests[$cycle]}"
        printf 'cycle_%s_payload_files=%s\n' "$cycle" "${cycle_payload_files[$cycle]}"
        printf 'cycle_%s_payload_references=%s\n' "$cycle" "${cycle_payload_references[$cycle]}"
        printf 'cycle_%s_restore_commit_id=%s\n' "$cycle" "${cycle_restore_commit_ids[$cycle]}"
    done
    printf 'elapsed_seconds=%s\n' "$elapsed_seconds"
    if [[ "$keep_tmp" == "1" ]]; then
        printf 'output_dir=%s\n' "$run_dir"
        printf 'log_file=%s\n' "$log_file"
    else
        printf 'output_dir=removed_on_success\n'
        printf 'log_file=removed_on_success\n'
    fi
} | tee -a "$log_file"
