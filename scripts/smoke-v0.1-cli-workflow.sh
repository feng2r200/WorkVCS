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
    --description "Smoke CLI workflow task")"
task_id="$(value "$task_output" "task_entity_id")"
task_version_id="$(value "$task_output" "task_entity_version_id")"
head_commit_id="$(value "$task_output" "commit_id")"
expect_value "$task_output" "status" "pending"
expect_nonempty "$task_output" "changeset_id"

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

step "history and show-at"
history_output="$(run_workvcs \
    history "$store" \
    --branch "$branch_id" \
    --expected-entries 4)"
expect_value "$history_output" "start_commit_id" "$head_commit_id"
expect_value "$history_output" "entries" "4"
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

step "runnable and next"
runnable_output="$(run_workvcs \
    runnable tasks "$store" \
    --session "$session_id" \
    --expected-head "$head_commit_id" \
    --expected-candidates 1)"
expect_value "$runnable_output" "head_commit_id" "$head_commit_id"
expect_value "$runnable_output" "candidates" "1"
expect_value "$runnable_output" "candidate.0.task_entity_id" "$task_id"
expect_value "$runnable_output" "candidate.0.runnable" "true"
expect_value "$runnable_output" "candidate.0.claim" "unclaimed"
expect_value "$runnable_output" "head_matches_expected" "true"
expect_value "$runnable_output" "candidates_match_expected" "true"

guard_output="$(run_workvcs \
    claim guard "$store" \
    --session "$session_id" \
    --task "$task_id" \
    --expected-allowed true \
    --expected-reason unclaimed \
    --expected-active-claims 0)"
expect_value "$guard_output" "allowed" "true"
expect_value "$guard_output" "reason" "unclaimed"
expect_value "$guard_output" "allowed_match_expected" "true"
expect_value "$guard_output" "reason_match_expected" "true"
expect_value "$guard_output" "active_claims_match_expected" "true"

next_output="$(run_workvcs \
    next "$store" \
    --session "$session_id" \
    --expected-selected true \
    --expected-head "$head_commit_id" \
    --expected-inspected-candidates 1 \
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

claimed_runnable_output="$(run_workvcs \
    runnable tasks "$store" \
    --session "$session_id" \
    --expected-head "$head_commit_id" \
    --expected-candidates 1)"
expect_value "$claimed_runnable_output" "candidate.0.claim" "claimed_by_session:${claim_id}"
expect_value "$claimed_runnable_output" "candidates_match_expected" "true"

printf 'smoke_result=passed\n'
printf 'store_id=%s\n' "$store_id"
printf 'workspace_id=%s\n' "$workspace_id"
printf 'branch_id=%s\n' "$branch_id"
printf 'genesis_state_digest=%s\n' "$genesis_state_digest"
printf 'task_entity_id=%s\n' "$task_id"
printf 'verification_entity_id=%s\n' "$verification_id"
printf 'claim_id=%s\n' "$claim_id"
