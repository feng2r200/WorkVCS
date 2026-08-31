#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/workvcs-cli-smoke.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT

store="$tmp_dir/workvcs.sqlite"
observation_file="$tmp_dir/observation.bin"
observation_detail_file="$tmp_dir/observation-detail.txt"
printf 'baseline bytes' >"$observation_file"
printf 'detail bytes' >"$observation_detail_file"

die() {
    printf 'smoke_error=%s\n' "$*" >&2
    exit 1
}

step() {
    printf 'smoke_step=%s\n' "$*" >&2
}

run_workvcs() {
    (cd "$repo_root" && cargo run -q -p workvcs-cli -- "$@")
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

expect_failure_contains() {
    local needle="$1"
    shift
    local output
    local status
    set +e
    output="$(run_workvcs "$@" 2>&1)"
    status=$?
    set -e
    [[ "$status" -ne 0 ]] || die "command unexpectedly succeeded: $*"
    [[ "$output" == *"$needle"* ]] || die "expected failure fragment ${needle}, got ${output}"
}

expect_integrity_matches() {
    local output="$1"
    expect_value "$output" "valid_required" "true"
    expect_value "$output" "checked_branches_match_expected" "true"
    expect_value "$output" "checked_commits_match_expected" "true"
    expect_value "$output" "checked_changesets_match_expected" "true"
    expect_value "$output" "checked_change_operations_match_expected" "true"
    expect_value "$output" "checked_changeset_causal_anchors_match_expected" "true"
    expect_value "$output" "checked_events_match_expected" "true"
    expect_value "$output" "checked_checkpoints_match_expected" "true"
    expect_value "$output" "invalid_checkpoints_match_expected" "true"
}

step "init"
init_output="$(run_workvcs \
    init "$store" \
    --display-name "smoke-store" \
    --expected-display-name "smoke-store" \
    --expected-store-format-version 1 \
    --expected-schema-version 1 \
    --expected-object-store-format-version 1 \
    --expected-id-scheme uuidv7-blob16 \
    --expected-digest-algorithm blake3-256 \
    --expected-canonical-json-profile workvcs-jcs-v1)"
expect_contains "$init_output" "initialized store_id="
expect_value "$init_output" "display_name_match_expected" "true"
expect_value "$init_output" "store_format_version_match_expected" "true"
expect_value "$init_output" "schema_version_match_expected" "true"
expect_value "$init_output" "object_store_format_version_match_expected" "true"
expect_value "$init_output" "id_scheme_match_expected" "true"
expect_value "$init_output" "digest_algorithm_match_expected" "true"
expect_value "$init_output" "canonical_json_profile_match_expected" "true"

step "store info"
store_output="$(run_workvcs \
    store info "$store" \
    --expected-display-name "smoke-store" \
    --expected-store-format-version 1 \
    --expected-schema-version 1 \
    --expected-object-store-format-version 1 \
    --expected-id-scheme uuidv7-blob16 \
    --expected-digest-algorithm blake3-256 \
    --expected-canonical-json-profile workvcs-jcs-v1)"
store_id="$(value "$store_output" "store_id")"
expect_nonempty "$store_output" "store_id"
expect_value "$store_output" "display_name" "smoke-store"
expect_value "$store_output" "display_name_match_expected" "true"
expect_value "$store_output" "canonical_json_profile_match_expected" "true"

step "workspace create"
workspace_output="$(run_workvcs \
    workspace create "$store" \
    --display-name "smoke-workspace")"
workspace_id="$(value "$workspace_output" "workspace_id")"
branch_id="$(value "$workspace_output" "branch_id")"
head_commit_id="$(value "$workspace_output" "genesis_commit_id")"
genesis_state_digest="$(value "$workspace_output" "state_digest")"
expect_value "$workspace_output" "branch_name" "main"
expect_nonempty "$workspace_output" "genesis_changeset_id"

step "task create"
task_output="$(run_workvcs \
    task create "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --description "Smoke CLI workflow task" \
    --priority 7)"
task_id="$(value "$task_output" "task_entity_id")"
task_version_id="$(value "$task_output" "task_entity_version_id")"
head_commit_id="$(value "$task_output" "commit_id")"
expect_value "$task_output" "status" "pending"
expect_nonempty "$task_output" "changeset_id"

step "prerequisite task create"
prerequisite_task_output="$(run_workvcs \
    task create "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --description "Smoke CLI prerequisite task" \
    --priority 1)"
prerequisite_task_id="$(value "$prerequisite_task_output" "task_entity_id")"
prerequisite_task_version_id="$(value "$prerequisite_task_output" "task_entity_version_id")"
head_commit_id="$(value "$prerequisite_task_output" "commit_id")"
expect_value "$prerequisite_task_output" "status" "pending"
expect_nonempty "$prerequisite_task_output" "changeset_id"

step "task scheduling"
dependency_output="$(run_workvcs \
    task depends-on "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --task "$task_id" \
    --depends-on "$prerequisite_task_id")"
dependency_relation_id="$(value "$dependency_output" "relation_id")"
head_commit_id="$(value "$dependency_output" "commit_id")"
expect_value "$dependency_output" "relation_type" "depends_on"
expect_value "$dependency_output" "source_task_entity_id" "$task_id"
expect_value "$dependency_output" "target_task_entity_id" "$prerequisite_task_id"

order_output="$(run_workvcs \
    task ordered-before "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --earlier "$prerequisite_task_id" \
    --later "$task_id")"
order_relation_id="$(value "$order_output" "relation_id")"
head_commit_id="$(value "$order_output" "commit_id")"
expect_value "$order_output" "relation_type" "ordered_before"
expect_value "$order_output" "source_task_entity_id" "$prerequisite_task_id"
expect_value "$order_output" "target_task_entity_id" "$task_id"

scheduling_output="$(run_workvcs \
    task scheduling-list "$store" \
    --branch "$branch_id" \
    --expected-relations 2)"
expect_value "$scheduling_output" "commit_id" "$head_commit_id"
expect_value "$scheduling_output" "relations" "2"
expect_value "$scheduling_output" "relations_match_expected" "true"
expect_value "$scheduling_output" "relation.0.relation_id" "$dependency_relation_id"
expect_value "$scheduling_output" "relation.0.relation_type" "depends_on"
expect_value "$scheduling_output" "relation.0.source_task_entity_id" "$task_id"
expect_value "$scheduling_output" "relation.0.target_task_entity_id" "$prerequisite_task_id"
expect_value "$scheduling_output" "relation.1.relation_id" "$order_relation_id"
expect_value "$scheduling_output" "relation.1.relation_type" "ordered_before"
expect_value "$scheduling_output" "relation.1.source_task_entity_id" "$prerequisite_task_id"
expect_value "$scheduling_output" "relation.1.target_task_entity_id" "$task_id"

step "acceptance criterion create"
criterion_output="$(run_workvcs \
    ac create "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --task "$task_id" \
    --task-version "$task_version_id" \
    --local-key "AC-SMOKE-1" \
    --statement "The WorkVCS CLI smoke workflow completes.")"
criterion_id="$(value "$criterion_output" "acceptance_criterion_entity_id")"
task_version_id="$(value "$criterion_output" "task_entity_version_id")"
head_commit_id="$(value "$criterion_output" "commit_id")"
expect_value "$criterion_output" "classification" "required"

step "acceptance criterion status before verification"
status_output="$(run_workvcs \
    ac status "$store" \
    --branch "$branch_id" \
    --criterion "$criterion_id")"
expect_value "$status_output" "status" "unverified"

step "task completion gate before verification"
expect_failure_contains \
    "unverified" \
    task transition "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --task "$task_id" \
    --task-version "$task_version_id" \
    --status done \
    --outcome completed

step "session start"
session_output="$(run_workvcs \
    session start "$store" \
    --workspace "$workspace_id" \
    --branch "$branch_id" \
    --metadata-json '{"purpose":"cli-smoke"}' \
    --expected-workspace "$workspace_id" \
    --expected-branch "$branch_id" \
    --expected-lifecycle-state active)"
session_id="$(value "$session_output" "session_id")"
expect_value "$session_output" "workspace_match_expected" "true"
expect_value "$session_output" "branch_match_expected" "true"
expect_value "$session_output" "lifecycle_state_match_expected" "true"

step "evidence create and show"
evidence_output="$(run_workvcs \
    evidence create "$store" \
    --kind manual-review \
    --metadata-json '{"summary":"smoke evidence"}' \
    --source-session "$session_id" \
    --content-role log \
    --content "smoke evidence bytes" \
    --media-type text/plain)"
evidence_id="$(value "$evidence_output" "evidence_id")"
evidence_digest="$(value "$evidence_output" "content.0.content_digest")"
expect_value "$evidence_output" "evidence_kind" "manual-review"
expect_value "$evidence_output" "contents" "1"
expect_value "$evidence_output" "source_session_id" "$session_id"

evidence_show_output="$(run_workvcs \
    evidence show "$store" \
    --evidence "$evidence_id" \
    --expected-content-digest "$evidence_digest")"
expect_value "$evidence_show_output" "evidence_id" "$evidence_id"
expect_value "$evidence_show_output" "content_matches_expected" "true"

step "resource observe"
resource_output="$(run_workvcs \
    resource create "$store" \
    --kind git)"
resource_id="$(value "$resource_output" "resource_id")"
expect_value "$resource_output" "resource_kind" "git"

observation_output="$(run_workvcs \
    resource observe "$store" \
    --resource "$resource_id" \
    --adapter-kind git \
    --adapter-schema-version 1 \
    --content-file "$observation_file" \
    --detail-content-file "$observation_detail_file" \
    --detail-media-type text/plain \
    --detail-format-metadata-json '{"encoding":"utf-8"}' \
    --source-session "$session_id")"
observation_id="$(value "$observation_output" "observation_id")"
fingerprint="$(value "$observation_output" "fingerprint")"
expect_value "$observation_output" "resource_id" "$resource_id"

observation_show_output="$(run_workvcs \
    resource observation-show "$store" \
    --observation "$observation_id")"
detail_digest="$(value "$observation_show_output" "detail.content_digest")"
expect_value "$observation_show_output" "observation_id" "$observation_id"
expect_value "$observation_show_output" "detail_content_present" "true"
expect_value "$observation_show_output" "detail.size_bytes" "12"
expect_value "$observation_show_output" "source_session_id" "$session_id"

observation_expected_output="$(run_workvcs \
    resource observation-show "$store" \
    --observation "$observation_id" \
    --expected-detail-content-digest "$detail_digest")"
expect_value "$observation_expected_output" "detail_content_matches_expected" "true"

observation_list_output="$(run_workvcs \
    resource observation-list "$store" \
    --resource "$resource_id" \
    --source-session "$session_id" \
    --expected-observations 1)"
expect_value "$observation_list_output" "observations" "1"
expect_value "$observation_list_output" "observations_match_expected" "true"
expect_value "$observation_list_output" "observation.0.observation_id" "$observation_id"

step "verification record"
verification_output="$(run_workvcs \
    verification record "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --acceptance-criterion "$criterion_id" \
    --result passed \
    --method manual-review \
    --evidence "$evidence_id" \
    --resource "$resource_id" \
    --adapter-kind git \
    --adapter-schema-version 1 \
    --scope-kind path \
    --scope-schema-version 1 \
    --scope-payload-json '{"path":"src/lib.rs"}' \
    --baseline-fingerprint "$fingerprint" \
    --baseline-observation "$observation_id" \
    --expected-branch "$branch_id" \
    --expected-head "$head_commit_id" \
    --expected-target-kind acceptance_criterion \
    --expected-target "$criterion_id" \
    --expected-result passed \
    --expected-evidence-relations 1)"
verification_id="$(value "$verification_output" "verification_entity_id")"
head_commit_id="$(value "$verification_output" "commit_id")"
expect_value "$verification_output" "branch_match_expected" "true"
expect_value "$verification_output" "head_match_expected" "true"
expect_value "$verification_output" "target_kind_match_expected" "true"
expect_value "$verification_output" "target_match_expected" "true"
expect_value "$verification_output" "result_match_expected" "true"
expect_value "$verification_output" "evidence_relations_match_expected" "true"

verification_list_output="$(run_workvcs \
    verification list "$store" \
    --branch "$branch_id" \
    --resource "$resource_id" \
    --baseline-observation "$observation_id" \
    --baseline-fingerprint "$fingerprint" \
    --expected-verifications 1)"
expect_value "$verification_list_output" "verifications" "1"
expect_value "$verification_list_output" "verifications_match_expected" "true"
expect_value "$verification_list_output" "verification.0.verification_entity_id" "$verification_id"

step "verification cache"
cache_record_output="$(run_workvcs \
    verification cache-record "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --verification "$verification_id" \
    --adapter-kind git \
    --adapter-schema-version 1 \
    --scope-schema-version 1 \
    --observation-status observed \
    --observed-fingerprint "$fingerprint" \
    --observation "$observation_id" \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability applicable \
    --expected-reason-code all_basis_applicable \
    --expected-resource-stamps 1)"
expect_value "$cache_record_output" "evaluated_commit_matches_expected" "true"
expect_value "$cache_record_output" "applicability_matches_expected" "true"
expect_value "$cache_record_output" "reason_code_matches_expected" "true"
expect_value "$cache_record_output" "resource_stamps_match_expected" "true"

cache_show_output="$(run_workvcs \
    verification cache-show "$store" \
    --branch "$branch_id" \
    --verification "$verification_id" \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability applicable \
    --expected-reason-code all_basis_applicable)"
expect_value "$cache_show_output" "cache_found" "true"
expect_value "$cache_show_output" "evaluated_commit_matches_expected" "true"
expect_value "$cache_show_output" "applicability_matches_expected" "true"
expect_value "$cache_show_output" "reason_code_matches_expected" "true"

cache_list_output="$(run_workvcs \
    verification cache-list "$store" \
    --branch "$branch_id" \
    --verification "$verification_id" \
    --expected-caches 1)"
expect_value "$cache_list_output" "caches" "1"
expect_value "$cache_list_output" "caches_match_expected" "true"
expect_value "$cache_list_output" "cache.0.verification_entity_id" "$verification_id"

step "acceptance criterion status after applicable cache"
verified_status_output="$(run_workvcs \
    ac status "$store" \
    --branch "$branch_id" \
    --criterion "$criterion_id")"
expect_value "$verified_status_output" "status" "verified"

step "history and show-at"
history_output="$(run_workvcs \
    history "$store" \
    --branch "$branch_id" \
    --expected-entries 7)"
expect_value "$history_output" "start_commit_id" "$head_commit_id"
expect_value "$history_output" "entries" "7"
expect_value "$history_output" "entries_match_expected" "true"
expect_contains "$history_output" "operation=verification.record"

show_at_output="$(run_workvcs \
    show-at "$store" \
    --branch "$branch_id")"
state_digest="$(value "$show_at_output" "state_digest")"
expect_value "$show_at_output" "workspace_id" "$workspace_id"
expect_value "$show_at_output" "commit_id" "$head_commit_id"
expect_contains "$show_at_output" "entities="

show_at_expected_output="$(run_workvcs \
    show-at "$store" \
    --commit "$head_commit_id" \
    --expected-state-digest "$state_digest")"
expect_value "$show_at_expected_output" "matches_expected" "true"

step "runnable scheduling"
runnable_output="$(run_workvcs \
    runnable tasks "$store" \
    --session "$session_id" \
    --expected-head "$head_commit_id" \
    --expected-candidates 2)"
expect_value "$runnable_output" "head_commit_id" "$head_commit_id"
expect_value "$runnable_output" "candidates" "2"
expect_value "$runnable_output" "candidate.0.task_entity_id" "$prerequisite_task_id"
expect_value "$runnable_output" "candidate.0.status" "pending"
expect_value "$runnable_output" "candidate.0.priority" "1"
expect_value "$runnable_output" "candidate.0.runnable" "true"
expect_value "$runnable_output" "candidate.0.lifecycle_eligible" "true"
expect_value "$runnable_output" "candidate.0.dependency_ready" "true"
expect_value "$runnable_output" "candidate.0.claim" "unclaimed"
expect_value "$runnable_output" "candidate.0.unsatisfied_dependencies" ""
expect_value "$runnable_output" "candidate.0.blocked_reasons" ""
expect_value "$runnable_output" "candidate.1.task_entity_id" "$task_id"
expect_value "$runnable_output" "candidate.1.status" "pending"
expect_value "$runnable_output" "candidate.1.priority" "7"
expect_value "$runnable_output" "candidate.1.runnable" "false"
expect_value "$runnable_output" "candidate.1.lifecycle_eligible" "true"
expect_value "$runnable_output" "candidate.1.dependency_ready" "false"
expect_value "$runnable_output" "candidate.1.claim" "unclaimed"
expect_value "$runnable_output" "candidate.1.unsatisfied_dependencies" "$prerequisite_task_id"
expect_value "$runnable_output" "candidate.1.blocked_reasons" "dependency_blocked"
expect_value "$runnable_output" "head_matches_expected" "true"
expect_value "$runnable_output" "candidates_match_expected" "true"

guard_output="$(run_workvcs \
    claim guard "$store" \
    --session "$session_id" \
    --task "$prerequisite_task_id" \
    --expected-allowed true \
    --expected-reason unclaimed \
    --expected-active-claims 0)"
expect_value "$guard_output" "allowed" "true"
expect_value "$guard_output" "reason" "unclaimed"
expect_value "$guard_output" "allowed_match_expected" "true"
expect_value "$guard_output" "reason_match_expected" "true"
expect_value "$guard_output" "active_claims_match_expected" "true"

claim_next_output="$(run_workvcs \
    claim next "$store" \
    --session "$session_id" \
    --expected-selected true \
    --expected-head "$head_commit_id" \
    --expected-inspected-candidates 2 \
    --expected-task "$prerequisite_task_id" \
    --expected-mode exclusive \
    --expected-lifecycle-state active)"
prerequisite_claim_id="$(value "$claim_next_output" "claim_id")"
expect_value "$claim_next_output" "selected" "true"
expect_value "$claim_next_output" "task_entity_id" "$prerequisite_task_id"
expect_value "$claim_next_output" "selected_match_expected" "true"
expect_value "$claim_next_output" "head_match_expected" "true"
expect_value "$claim_next_output" "inspected_candidates_match_expected" "true"
expect_value "$claim_next_output" "task_match_expected" "true"
expect_value "$claim_next_output" "mode_match_expected" "true"
expect_value "$claim_next_output" "lifecycle_state_match_expected" "true"

claimed_runnable_output="$(run_workvcs \
    runnable tasks "$store" \
    --session "$session_id" \
    --expected-head "$head_commit_id" \
    --expected-candidates 1)"
expect_value "$claimed_runnable_output" "candidate.0.task_entity_id" "$prerequisite_task_id"
expect_value "$claimed_runnable_output" "candidate.0.claim" "claimed_by_session:${prerequisite_claim_id}"
expect_value "$claimed_runnable_output" "candidates_match_expected" "true"

step "prerequisite completion"
prerequisite_completion_output="$(run_workvcs \
    task transition "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --task "$prerequisite_task_id" \
    --task-version "$prerequisite_task_version_id" \
    --status done \
    --outcome completed \
    --session "$session_id")"
prerequisite_task_version_id="$(value "$prerequisite_completion_output" "task_entity_version_id")"
head_commit_id="$(value "$prerequisite_completion_output" "commit_id")"
expect_value "$prerequisite_completion_output" "task_entity_id" "$prerequisite_task_id"
expect_value "$prerequisite_completion_output" "status" "done"

prerequisite_claim_release_output="$(run_workvcs \
    claim release "$store" \
    --session "$session_id" \
    --claim "$prerequisite_claim_id" \
    --expected-session "$session_id" \
    --expected-lifecycle-state released)"
expect_value "$prerequisite_claim_release_output" "claim_id" "$prerequisite_claim_id"
expect_value "$prerequisite_claim_release_output" "session_id" "$session_id"
expect_value "$prerequisite_claim_release_output" "lifecycle_state" "released"
expect_value "$prerequisite_claim_release_output" "session_match_expected" "true"
expect_value "$prerequisite_claim_release_output" "lifecycle_state_match_expected" "true"

focus_clear_output="$(run_workvcs \
    session focus-clear "$store" \
    --session "$session_id" \
    --expected-session "$session_id" \
    --expected-focus none \
    --expected-focus-path-entries 0 \
    --expected-lifecycle-state active)"
expect_value "$focus_clear_output" "session_id" "$session_id"
expect_value "$focus_clear_output" "focus_entity_id" "none"
expect_value "$focus_clear_output" "focus_path_entries" "0"
expect_value "$focus_clear_output" "session_match_expected" "true"
expect_value "$focus_clear_output" "focus_match_expected" "true"
expect_value "$focus_clear_output" "focus_path_entries_match_expected" "true"
expect_value "$focus_clear_output" "lifecycle_state_match_expected" "true"

step "verification refresh after prerequisite completion"
stale_status_output="$(run_workvcs \
    ac status "$store" \
    --branch "$branch_id" \
    --criterion "$criterion_id")"
expect_value "$stale_status_output" "status" "stale"

verification_refresh_output="$(run_workvcs \
    verification record "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --acceptance-criterion "$criterion_id" \
    --result passed \
    --method manual-review \
    --evidence "$evidence_id" \
    --resource "$resource_id" \
    --adapter-kind git \
    --adapter-schema-version 1 \
    --scope-kind path \
    --scope-schema-version 1 \
    --scope-payload-json '{"path":"src/lib.rs"}' \
    --baseline-fingerprint "$fingerprint" \
    --baseline-observation "$observation_id" \
    --expected-branch "$branch_id" \
    --expected-head "$head_commit_id" \
    --expected-target-kind acceptance_criterion \
    --expected-target "$criterion_id" \
    --expected-result passed \
    --expected-evidence-relations 1)"
verification_id="$(value "$verification_refresh_output" "verification_entity_id")"
head_commit_id="$(value "$verification_refresh_output" "commit_id")"
expect_value "$verification_refresh_output" "branch_match_expected" "true"
expect_value "$verification_refresh_output" "head_match_expected" "true"
expect_value "$verification_refresh_output" "target_kind_match_expected" "true"
expect_value "$verification_refresh_output" "target_match_expected" "true"
expect_value "$verification_refresh_output" "result_match_expected" "true"
expect_value "$verification_refresh_output" "evidence_relations_match_expected" "true"

cache_refresh_output="$(run_workvcs \
    verification cache-record "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --verification "$verification_id" \
    --adapter-kind git \
    --adapter-schema-version 1 \
    --scope-schema-version 1 \
    --observation-status observed \
    --observed-fingerprint "$fingerprint" \
    --observation "$observation_id" \
    --expected-evaluated-commit "$head_commit_id" \
    --expected-applicability applicable \
    --expected-reason-code all_basis_applicable \
    --expected-resource-stamps 1)"
expect_value "$cache_refresh_output" "evaluated_commit_matches_expected" "true"
expect_value "$cache_refresh_output" "applicability_matches_expected" "true"
expect_value "$cache_refresh_output" "reason_code_matches_expected" "true"
expect_value "$cache_refresh_output" "resource_stamps_match_expected" "true"

refreshed_status_output="$(run_workvcs \
    ac status "$store" \
    --branch "$branch_id" \
    --criterion "$criterion_id")"
expect_value "$refreshed_status_output" "status" "verified"

ready_runnable_output="$(run_workvcs \
    runnable tasks "$store" \
    --session "$session_id" \
    --expected-head "$head_commit_id" \
    --expected-candidates 2)"
expect_value "$ready_runnable_output" "candidate.0.task_entity_id" "$task_id"
expect_value "$ready_runnable_output" "candidate.0.priority" "7"
expect_value "$ready_runnable_output" "candidate.0.runnable" "true"
expect_value "$ready_runnable_output" "candidate.0.dependency_ready" "true"
expect_value "$ready_runnable_output" "candidate.0.claim" "unclaimed"
expect_value "$ready_runnable_output" "candidate.1.task_entity_id" "$prerequisite_task_id"
expect_value "$ready_runnable_output" "candidate.1.status" "done"
expect_value "$ready_runnable_output" "candidate.1.runnable" "false"
expect_value "$ready_runnable_output" "candidate.1.blocked_reasons" "lifecycle_ineligible"
expect_value "$ready_runnable_output" "candidates_match_expected" "true"

step "next dependent task"
next_output="$(run_workvcs \
    next "$store" \
    --session "$session_id" \
    --expected-selected true \
    --expected-head "$head_commit_id" \
    --expected-inspected-candidates 2 \
    --expected-task "$task_id" \
    --expected-mode exclusive \
    --expected-lifecycle-state active)"
claim_id="$(value "$next_output" "claim_id")"
expect_value "$next_output" "selected" "true"
expect_value "$next_output" "task_entity_id" "$task_id"
expect_value "$next_output" "context_focus_entity_id" "$task_id"
expect_value "$next_output" "context_runnable_candidates" "1"
expect_value "$next_output" "selected_match_expected" "true"
expect_value "$next_output" "head_match_expected" "true"
expect_value "$next_output" "inspected_candidates_match_expected" "true"
expect_value "$next_output" "task_match_expected" "true"
expect_value "$next_output" "mode_match_expected" "true"
expect_value "$next_output" "lifecycle_state_match_expected" "true"

step "task completion after verification"
completion_output="$(run_workvcs \
    task transition "$store" \
    --branch "$branch_id" \
    --head "$head_commit_id" \
    --task "$task_id" \
    --task-version "$task_version_id" \
    --status done \
    --outcome completed \
    --session "$session_id")"
completed_task_state_digest="$(value "$completion_output" "task_state_digest")"
head_commit_id="$(value "$completion_output" "commit_id")"
task_version_id="$(value "$completion_output" "task_entity_version_id")"
expect_value "$completion_output" "status" "done"

completed_task_output="$(run_workvcs \
    task show "$store" \
    --branch "$branch_id" \
    --task "$task_id" \
    --expected-state-digest "$completed_task_state_digest")"
expect_value "$completed_task_output" "task_entity_version_id" "$task_version_id"
expect_value "$completed_task_output" "status" "done"
expect_value "$completed_task_output" "priority" "7"
expect_value "$completed_task_output" "matches_expected" "true"

completed_runnable_output="$(run_workvcs \
    runnable tasks "$store" \
    --session "$session_id" \
    --task "$task_id" \
    --expected-head "$head_commit_id" \
    --expected-candidates 1)"
expect_value "$completed_runnable_output" "candidate.0.task_entity_id" "$task_id"
expect_value "$completed_runnable_output" "candidate.0.status" "done"
expect_value "$completed_runnable_output" "candidate.0.priority" "7"
expect_value "$completed_runnable_output" "candidate.0.runnable" "false"
expect_value "$completed_runnable_output" "candidate.0.lifecycle_eligible" "false"
expect_value "$completed_runnable_output" "head_matches_expected" "true"
expect_value "$completed_runnable_output" "candidates_match_expected" "true"

completion_history_output="$(run_workvcs \
    history "$store" \
    --branch "$branch_id" \
    --expected-entries 10)"
expect_value "$completion_history_output" "start_commit_id" "$head_commit_id"
expect_value "$completion_history_output" "entries" "10"
expect_value "$completion_history_output" "entries_match_expected" "true"
expect_contains "$completion_history_output" "operation=entity.transition"

step "runtime closeout"
claim_release_output="$(run_workvcs \
    claim release "$store" \
    --session "$session_id" \
    --claim "$claim_id" \
    --expected-session "$session_id" \
    --expected-lifecycle-state released)"
expect_value "$claim_release_output" "claim_id" "$claim_id"
expect_value "$claim_release_output" "session_id" "$session_id"
expect_value "$claim_release_output" "lifecycle_state" "released"
expect_value "$claim_release_output" "session_match_expected" "true"
expect_value "$claim_release_output" "lifecycle_state_match_expected" "true"

session_end_output="$(run_workvcs \
    session end "$store" \
    --session "$session_id" \
    --summary-json '{"outcome":"smoke-complete"}' \
    --expected-session "$session_id" \
    --expected-lifecycle-state ended)"
session_diff_id="$(value "$session_end_output" "session_diff_id")"
expect_value "$session_end_output" "session_id" "$session_id"
expect_value "$session_end_output" "lifecycle_state" "ended"
expect_value "$session_end_output" "session_match_expected" "true"
expect_value "$session_end_output" "lifecycle_state_match_expected" "true"
expect_nonempty "$session_end_output" "session_diff_id"

step "integrity gate"
store_integrity_output="$(run_workvcs \
    store integrity "$store" \
    --require-valid \
    --expected-checked-branches 1 \
    --expected-checked-commits 10 \
    --expected-checked-changesets 10 \
    --expected-checked-change-operations 14 \
    --expected-checked-changeset-causal-anchors 0 \
    --expected-checked-events 19 \
    --expected-checked-checkpoints 0 \
    --expected-invalid-checkpoints 0)"
expect_value "$store_integrity_output" "checked_branches" "1"
expect_value "$store_integrity_output" "checked_commits" "10"
expect_value "$store_integrity_output" "checked_changesets" "10"
expect_value "$store_integrity_output" "checked_change_operations" "14"
expect_value "$store_integrity_output" "checked_changeset_causal_anchors" "0"
expect_value "$store_integrity_output" "checked_events" "19"
expect_value "$store_integrity_output" "checked_checkpoints" "0"
expect_value "$store_integrity_output" "invalid_checkpoints" "0"
expect_integrity_matches "$store_integrity_output"

doctor_output="$(run_workvcs \
    doctor "$store" \
    --require-valid \
    --expected-checked-branches 1 \
    --expected-checked-commits 10 \
    --expected-checked-changesets 10 \
    --expected-checked-change-operations 14 \
    --expected-checked-changeset-causal-anchors 0 \
    --expected-checked-events 19 \
    --expected-checked-checkpoints 0 \
    --expected-invalid-checkpoints 0)"
expect_contains "$doctor_output" "ok store_id=$store_id schema_version=1 canonical_json_profile=workvcs-jcs-v1"
expect_contains "$doctor_output" "checked_branches=1 checked_commits=10 checked_changesets=10 checked_change_operations=14 checked_changeset_causal_anchors=0 checked_events=19 checked_checkpoints=0 invalid_checkpoints=0"
expect_integrity_matches "$doctor_output"

printf 'smoke_result=passed\n'
printf 'store_id=%s\n' "$store_id"
printf 'workspace_id=%s\n' "$workspace_id"
printf 'branch_id=%s\n' "$branch_id"
printf 'genesis_state_digest=%s\n' "$genesis_state_digest"
printf 'task_entity_id=%s\n' "$task_id"
printf 'verification_entity_id=%s\n' "$verification_id"
printf 'claim_id=%s\n' "$claim_id"
printf 'session_diff_id=%s\n' "$session_diff_id"
