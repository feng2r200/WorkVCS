#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/workvcs-cli-smoke.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT

store="$tmp_dir/workvcs.sqlite"
bundle_source_store="$tmp_dir/bundle-source.sqlite"
bundle_target_store="$tmp_dir/bundle-target.sqlite"
bundle_dir="$tmp_dir/bundle-export"
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

step "task claim release before merge"
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

step "merge lifecycle"
merge_base_commit_id="$head_commit_id"
source_branch_output="$(run_workvcs \
    branch fork "$store" \
    --from-branch "$branch_id" \
    --name "smoke-source")"
source_branch_id="$(value "$source_branch_output" "branch_id")"
expect_value "$source_branch_output" "workspace_id" "$workspace_id"
expect_value "$source_branch_output" "source_branch_id" "$branch_id"
expect_value "$source_branch_output" "head_commit_id" "$merge_base_commit_id"

branch_list_output="$(run_workvcs \
    branch list "$store" \
    --workspace "$workspace_id" \
    --expected-branches 2)"
expect_value "$branch_list_output" "branches" "2"
expect_value "$branch_list_output" "branches_match_expected" "true"

target_merge_task_output="$(run_workvcs \
    task create "$store" \
    --branch "$branch_id" \
    --head "$merge_base_commit_id" \
    --description "Smoke CLI target merge marker" \
    --priority 3)"
target_merge_task_id="$(value "$target_merge_task_output" "task_entity_id")"
target_merge_head_id="$(value "$target_merge_task_output" "commit_id")"
head_commit_id="$target_merge_head_id"
expect_value "$target_merge_task_output" "status" "pending"
expect_nonempty "$target_merge_task_output" "changeset_id"

source_merge_task_output="$(run_workvcs \
    task create "$store" \
    --branch "$source_branch_id" \
    --head "$merge_base_commit_id" \
    --description "Smoke CLI source merge task" \
    --priority 2)"
source_merge_task_id="$(value "$source_merge_task_output" "task_entity_id")"
source_merge_head_id="$(value "$source_merge_task_output" "commit_id")"
expect_value "$source_merge_task_output" "status" "pending"
expect_nonempty "$source_merge_task_output" "changeset_id"

merge_start_output="$(run_workvcs \
    merge start "$store" \
    --target-branch "$branch_id" \
    --source-branch "$source_branch_id" \
    --session "$session_id" \
    --expected-runtime-state active \
    --expected-merge-base "$merge_base_commit_id" \
    --expected-target-head "$target_merge_head_id" \
    --expected-source-head "$source_merge_head_id" \
    --expected-origin-session "$session_id")"
merge_id="$(value "$merge_start_output" "merge_id")"
expect_value "$merge_start_output" "runtime_state" "active"
expect_value "$merge_start_output" "runtime_state_matches_expected" "true"
expect_value "$merge_start_output" "merge_base_matches_expected" "true"
expect_value "$merge_start_output" "target_head_matches_expected" "true"
expect_value "$merge_start_output" "source_head_matches_expected" "true"
expect_value "$merge_start_output" "origin_session_matches_expected" "true"

merge_show_output="$(run_workvcs \
    merge show "$store" \
    --merge "$merge_id" \
    --expected-runtime-state active \
    --expected-outcome none)"
merge_item_id="$(value "$merge_show_output" "item.0.merge_item_id")"
expect_value "$merge_show_output" "items" "1"
expect_value "$merge_show_output" "item.0.classification" "AUTO"
expect_value "$merge_show_output" "item.0.subject_id" "$source_merge_task_id"
expect_value "$merge_show_output" "item.0.resolution" "none"
expect_value "$merge_show_output" "runtime_state_matches_expected" "true"
expect_value "$merge_show_output" "outcome_matches_expected" "true"

merge_resolve_output="$(run_workvcs \
    merge resolve "$store" \
    --item "$merge_item_id" \
    --kind theirs \
    --session "$session_id" \
    --rationale-json '{"reason":"accept smoke source task"}' \
    --expected-merge "$merge_id" \
    --expected-resolution theirs \
    --expected-resolved-by-session "$session_id")"
expect_value "$merge_resolve_output" "merge_matches_expected" "true"
expect_value "$merge_resolve_output" "resolution_matches_expected" "true"
expect_value "$merge_resolve_output" "resolved_by_session_matches_expected" "true"

merge_freeze_output="$(run_workvcs \
    merge freeze "$store" \
    --merge "$merge_id" \
    --expected-merge "$merge_id" \
    --expected-frozen-items 1)"
expect_value "$merge_freeze_output" "merge_matches_expected" "true"
expect_value "$merge_freeze_output" "frozen_items_match_expected" "true"

merge_active_list_output="$(run_workvcs \
    merge list "$store" \
    --workspace "$workspace_id" \
    --target-branch "$branch_id" \
    --expected-merges 1)"
expect_value "$merge_active_list_output" "merges" "1"
expect_value "$merge_active_list_output" "merge.0.merge_id" "$merge_id"
expect_value "$merge_active_list_output" "merge.0.runtime_state" "active"
expect_value "$merge_active_list_output" "merges_match_expected" "true"

merge_continue_output="$(run_workvcs \
    merge continue "$store" \
    --merge "$merge_id" \
    --session "$session_id" \
    --detail-json '{"reason":"complete smoke merge"}' \
    --expected-runtime-state completed \
    --expected-target-branch "$branch_id" \
    --expected-source-branch "$source_branch_id" \
    --expected-continued-by-session "$session_id")"
result_commit_id="$(value "$merge_continue_output" "result_commit_id")"
head_commit_id="$result_commit_id"
expect_value "$merge_continue_output" "runtime_state" "completed"
expect_value "$merge_continue_output" "runtime_state_matches_expected" "true"
expect_value "$merge_continue_output" "target_branch_matches_expected" "true"
expect_value "$merge_continue_output" "source_branch_matches_expected" "true"
expect_value "$merge_continue_output" "continued_by_session_matches_expected" "true"
expect_nonempty "$merge_continue_output" "changeset_id"

merge_head_output="$(run_workvcs \
    branch head "$store" \
    --branch "$branch_id")"
merge_state_digest="$(value "$merge_head_output" "state_digest")"
expect_value "$merge_head_output" "head_commit_id" "$result_commit_id"
expect_value "$merge_head_output" "head_commit_kind" "merge"
expect_value "$merge_head_output" "head_operation_type" "merge.continue"

merge_commit_output="$(run_workvcs \
    commit show "$store" \
    --commit "$result_commit_id")"
expect_value "$merge_commit_output" "commit_kind" "merge"
expect_value "$merge_commit_output" "operation_type" "merge.continue"
expect_value "$merge_commit_output" "parents" "2"
expect_value "$merge_commit_output" "parent[0].role" "primary"
expect_value "$merge_commit_output" "parent[0].commit_id" "$target_merge_head_id"
expect_value "$merge_commit_output" "parent[1].role" "secondary"
expect_value "$merge_commit_output" "parent[1].commit_id" "$source_merge_head_id"

merge_closed_show_output="$(run_workvcs \
    merge show "$store" \
    --merge "$merge_id" \
    --expected-runtime-state completed \
    --expected-outcome completed)"
expect_value "$merge_closed_show_output" "runtime_state" "completed"
expect_value "$merge_closed_show_output" "outcome" "completed"
expect_value "$merge_closed_show_output" "outcome.result_commit_id" "$result_commit_id"
expect_value "$merge_closed_show_output" "runtime_state_matches_expected" "true"
expect_value "$merge_closed_show_output" "outcome_matches_expected" "true"

merge_hidden_list_output="$(run_workvcs \
    merge list "$store" \
    --workspace "$workspace_id" \
    --target-branch "$branch_id" \
    --expected-merges 0)"
expect_value "$merge_hidden_list_output" "merges" "0"
expect_value "$merge_hidden_list_output" "merges_match_expected" "true"

merge_closed_list_output="$(run_workvcs \
    merge list "$store" \
    --workspace "$workspace_id" \
    --target-branch "$branch_id" \
    --include-closed \
    --expected-merges 1)"
expect_value "$merge_closed_list_output" "merges" "1"
expect_value "$merge_closed_list_output" "merge.0.merge_id" "$merge_id"
expect_value "$merge_closed_list_output" "merge.0.runtime_state" "completed"
expect_value "$merge_closed_list_output" "merge.0.outcome" "completed"
expect_value "$merge_closed_list_output" "merges_match_expected" "true"

merge_history_output="$(run_workvcs \
    history "$store" \
    --branch "$branch_id" \
    --expected-entries 12)"
expect_value "$merge_history_output" "start_commit_id" "$result_commit_id"
expect_value "$merge_history_output" "entries" "12"
expect_value "$merge_history_output" "entries_match_expected" "true"
expect_contains "$merge_history_output" "operation=merge.continue"

merge_show_at_output="$(run_workvcs \
    show-at "$store" \
    --branch "$branch_id")"
expect_value "$merge_show_at_output" "commit_id" "$result_commit_id"
expect_value "$merge_show_at_output" "state_digest" "$merge_state_digest"
expect_contains "$merge_show_at_output" "$target_merge_task_id"
expect_contains "$merge_show_at_output" "$source_merge_task_id"

merge_show_at_expected_output="$(run_workvcs \
    show-at "$store" \
    --commit "$result_commit_id" \
    --expected-state-digest "$merge_state_digest")"
expect_value "$merge_show_at_expected_output" "matches_expected" "true"

step "checkpoint gate"
checkpoint_output="$(run_workvcs \
    checkpoint create "$store" \
    --commit "$head_commit_id")"
checkpoint_id="$(value "$checkpoint_output" "checkpoint_id")"
checkpoint_content_digest="$(value "$checkpoint_output" "content_digest")"
expect_value "$checkpoint_output" "commit_id" "$head_commit_id"
expect_value "$checkpoint_output" "state_digest" "$merge_state_digest"
expect_value "$checkpoint_output" "checkpoint_format_version" "1"
expect_value "$checkpoint_output" "media_type" "application/vnd.workvcs.workstate-checkpoint+json"
expect_value "$checkpoint_output" "usability_state" "usable"

checkpoint_show_output="$(run_workvcs \
    checkpoint show "$store" \
    --checkpoint "$checkpoint_id" \
    --expected-state-digest "$merge_state_digest" \
    --expected-content-digest "$checkpoint_content_digest")"
expect_value "$checkpoint_show_output" "checkpoint_id" "$checkpoint_id"
expect_value "$checkpoint_show_output" "commit_id" "$head_commit_id"
expect_value "$checkpoint_show_output" "state_matches_expected" "true"
expect_value "$checkpoint_show_output" "content_matches_expected" "true"

checkpoint_validate_output="$(run_workvcs \
    checkpoint validate "$store" \
    --checkpoint "$checkpoint_id" \
    --require-valid)"
expect_value "$checkpoint_validate_output" "checkpoint_id" "$checkpoint_id"
expect_value "$checkpoint_validate_output" "valid" "true"
expect_value "$checkpoint_validate_output" "problem" "none"
expect_value "$checkpoint_validate_output" "expected_state_digest" "$merge_state_digest"
expect_value "$checkpoint_validate_output" "expected_content_digest" "$checkpoint_content_digest"
expect_value "$checkpoint_validate_output" "valid_required" "true"

checkpoint_list_output="$(run_workvcs \
    checkpoint list "$store" \
    --commit "$head_commit_id" \
    --usability-state usable \
    --content-digest "$checkpoint_content_digest" \
    --expected-checkpoints 1)"
expect_value "$checkpoint_list_output" "commit_id" "$head_commit_id"
expect_value "$checkpoint_list_output" "checkpoints" "1"
expect_value "$checkpoint_list_output" "checkpoint[0].id" "$checkpoint_id"
expect_value "$checkpoint_list_output" "checkpoint[0].content_digest" "$checkpoint_content_digest"
expect_value "$checkpoint_list_output" "checkpoint[0].usability_state" "usable"
expect_value "$checkpoint_list_output" "checkpoints_match_expected" "true"

checkpoint_latest_output="$(run_workvcs \
    checkpoint latest "$store" \
    --commit "$head_commit_id" \
    --require-found \
    --expected-checkpoint "$checkpoint_id")"
expect_value "$checkpoint_latest_output" "commit_id" "$head_commit_id"
expect_value "$checkpoint_latest_output" "checkpoint_found" "true"
expect_value "$checkpoint_latest_output" "checkpoint_id" "$checkpoint_id"
expect_value "$checkpoint_latest_output" "checkpoint_found_required" "true"
expect_value "$checkpoint_latest_output" "checkpoint_matches_expected" "true"

step "bundle portability gate"
bundle_init_output="$(run_workvcs \
    init "$bundle_source_store" \
    --display-name "bundle-smoke-source")"
expect_contains "$bundle_init_output" "initialized store_id="

bundle_workspace_output="$(run_workvcs \
    workspace create "$bundle_source_store" \
    --display-name "bundle-smoke-workspace")"
bundle_branch_id="$(value "$bundle_workspace_output" "branch_id")"
bundle_genesis_commit_id="$(value "$bundle_workspace_output" "genesis_commit_id")"
expect_value "$bundle_workspace_output" "branch_name" "main"

bundle_first_task_output="$(run_workvcs \
    task create "$bundle_source_store" \
    --branch "$bundle_branch_id" \
    --head "$bundle_genesis_commit_id" \
    --description "Bundle smoke older target task" \
    --priority 4)"
bundle_first_commit_id="$(value "$bundle_first_task_output" "commit_id")"
expect_value "$bundle_first_task_output" "status" "pending"

cp "$bundle_source_store" "$bundle_target_store"

bundle_second_task_output="$(run_workvcs \
    task create "$bundle_source_store" \
    --branch "$bundle_branch_id" \
    --head "$bundle_first_commit_id" \
    --description "Bundle smoke exported task" \
    --priority 5)"
bundle_task_id="$(value "$bundle_second_task_output" "task_entity_id")"
bundle_head_id="$(value "$bundle_second_task_output" "commit_id")"
expect_value "$bundle_second_task_output" "status" "pending"

bundle_source_head_output="$(run_workvcs \
    branch head "$bundle_source_store" \
    --branch "$bundle_branch_id")"
bundle_state_digest="$(value "$bundle_source_head_output" "state_digest")"
expect_value "$bundle_source_head_output" "head_commit_id" "$bundle_head_id"

bundle_checkpoint_output="$(run_workvcs \
    checkpoint create "$bundle_source_store" \
    --commit "$bundle_head_id")"
bundle_checkpoint_id="$(value "$bundle_checkpoint_output" "checkpoint_id")"
bundle_checkpoint_content_digest="$(value "$bundle_checkpoint_output" "content_digest")"
expect_value "$bundle_checkpoint_output" "commit_id" "$bundle_head_id"
expect_value "$bundle_checkpoint_output" "state_digest" "$bundle_state_digest"
expect_value "$bundle_checkpoint_output" "checkpoint_format_version" "1"
expect_value "$bundle_checkpoint_output" "media_type" "application/vnd.workvcs.workstate-checkpoint+json"
expect_value "$bundle_checkpoint_output" "usability_state" "usable"

bundle_export_output="$(run_workvcs \
    bundle export "$bundle_source_store" \
    --commit "$bundle_head_id" \
    --expected-state-digest "$bundle_state_digest" \
    --expected-commits 3 \
    --expected-exported-branch-heads 1 \
    --expected-entities 2 \
    --expected-relations 0 \
    --expected-checkpoint-candidates 1)"
expect_value "$bundle_export_output" "commit_id" "$bundle_head_id"
expect_value "$bundle_export_output" "commits" "3"
expect_value "$bundle_export_output" "exported_branch_heads" "1"
expect_value "$bundle_export_output" "entities" "2"
expect_value "$bundle_export_output" "relations" "0"
expect_value "$bundle_export_output" "checkpoint_candidates" "1"
expect_value "$bundle_export_output" "state_matches_expected" "true"
expect_value "$bundle_export_output" "commits_match_expected" "true"
expect_value "$bundle_export_output" "exported_branch_heads_match_expected" "true"
expect_value "$bundle_export_output" "entities_match_expected" "true"
expect_value "$bundle_export_output" "relations_match_expected" "true"
expect_value "$bundle_export_output" "checkpoint_candidates_match_expected" "true"
expect_value "$bundle_export_output" "checkpoint_candidate[0].id" "$bundle_checkpoint_id"
expect_value "$bundle_export_output" "checkpoint_candidate[0].commit_id" "$bundle_head_id"
expect_value "$bundle_export_output" "checkpoint_candidate[0].state_digest" "$bundle_state_digest"
expect_value "$bundle_export_output" "checkpoint_candidate[0].content_digest" "$bundle_checkpoint_content_digest"
expect_value "$bundle_export_output" "checkpoint_candidate[0].usability_state" "usable"

bundle_export_dir_output="$(run_workvcs \
    bundle export-dir "$bundle_source_store" \
    --commit "$bundle_head_id" \
    --output-dir "$bundle_dir" \
    --expected-payload-files 7 \
    --expected-payload-references 17)"
expect_value "$bundle_export_dir_output" "commit_id" "$bundle_head_id"
expect_value "$bundle_export_dir_output" "payload_files" "7"
expect_value "$bundle_export_dir_output" "payload_references" "17"
expect_value "$bundle_export_dir_output" "payload_files_match_expected" "true"
expect_value "$bundle_export_dir_output" "payload_references_match_expected" "true"

bundle_validate_output="$(run_workvcs \
    bundle validate-dir "$bundle_source_store" \
    --commit "$bundle_head_id" \
    --input-dir "$bundle_dir" \
    --require-valid \
    --expected-payload-files 7 \
    --expected-payload-references 17)"
expect_value "$bundle_validate_output" "commit_id" "$bundle_head_id"
expect_value "$bundle_validate_output" "valid" "true"
expect_value "$bundle_validate_output" "valid_required" "true"
expect_value "$bundle_validate_output" "expected_payload_files" "7"
expect_value "$bundle_validate_output" "actual_payload_files" "7"
expect_value "$bundle_validate_output" "expected_payload_references" "17"
expect_value "$bundle_validate_output" "payload_files_match_expected" "true"
expect_value "$bundle_validate_output" "payload_references_match_expected" "true"
expect_value "$bundle_validate_output" "problem" "none"

bundle_preflight_output="$(run_workvcs \
    bundle preflight-dir "$bundle_target_store" \
    --input-dir "$bundle_dir" \
    --require-valid \
    --require-can-apply \
    --expected-payload-files 7 \
    --expected-payload-references 17 \
    --expected-exported-branch-heads 1 \
    --expected-branch-heads-already-present 0 \
    --expected-branch-heads-missing 0 \
    --expected-branch-heads-fast-forward 1 \
    --expected-branch-heads-diverged 0)"
expect_value "$bundle_preflight_output" "valid" "true"
expect_value "$bundle_preflight_output" "valid_required" "true"
expect_value "$bundle_preflight_output" "can_apply" "true"
expect_value "$bundle_preflight_output" "can_apply_required" "true"
expect_value "$bundle_preflight_output" "action" "same_store_fast_forward_ready"
expect_value "$bundle_preflight_output" "source_store_relation" "same_store"
expect_value "$bundle_preflight_output" "incoming_commit_present" "false"
expect_value "$bundle_preflight_output" "import_required" "true"
expect_value "$bundle_preflight_output" "payload_files_match_expected" "true"
expect_value "$bundle_preflight_output" "payload_references_match_expected" "true"
expect_value "$bundle_preflight_output" "exported_branch_heads_match_expected" "true"
expect_value "$bundle_preflight_output" "branch_heads_already_present_match_expected" "true"
expect_value "$bundle_preflight_output" "branch_heads_missing_match_expected" "true"
expect_value "$bundle_preflight_output" "branch_heads_fast_forward_match_expected" "true"
expect_value "$bundle_preflight_output" "branch_heads_diverged_match_expected" "true"

bundle_apply_output="$(run_workvcs \
    bundle apply-dir "$bundle_target_store" \
    --input-dir "$bundle_dir" \
    --require-applied \
    --expected-outcome same_store_fast_forward_applied \
    --expected-imported-commits 1 \
    --expected-imported-entity-versions 1 \
    --expected-imported-checkpoints 1 \
    --expected-imported-checkpoint-statuses 1 \
    --expected-updated-branch-heads 1)"
bundle_import_id="$(value "$bundle_apply_output" "import_id")"
bundle_digest="$(value "$bundle_apply_output" "bundle_digest")"
expect_value "$bundle_apply_output" "applied" "true"
expect_value "$bundle_apply_output" "applied_required" "true"
expect_value "$bundle_apply_output" "outcome" "same_store_fast_forward_applied"
expect_value "$bundle_apply_output" "imported_commits" "1"
expect_value "$bundle_apply_output" "imported_entity_versions" "1"
expect_value "$bundle_apply_output" "imported_checkpoints" "1"
expect_value "$bundle_apply_output" "imported_checkpoint_statuses" "1"
expect_value "$bundle_apply_output" "updated_branch_heads" "1"
expect_value "$bundle_apply_output" "outcome_matches_expected" "true"
expect_value "$bundle_apply_output" "imported_commits_match_expected" "true"
expect_value "$bundle_apply_output" "imported_entity_versions_match_expected" "true"
expect_value "$bundle_apply_output" "imported_checkpoints_match_expected" "true"
expect_value "$bundle_apply_output" "imported_checkpoint_statuses_match_expected" "true"
expect_value "$bundle_apply_output" "updated_branch_heads_match_expected" "true"

bundle_target_head_output="$(run_workvcs \
    branch head "$bundle_target_store" \
    --branch "$bundle_branch_id" \
    --expected-state-digest "$bundle_state_digest")"
expect_value "$bundle_target_head_output" "head_commit_id" "$bundle_head_id"
expect_value "$bundle_target_head_output" "state_digest" "$bundle_state_digest"
expect_value "$bundle_target_head_output" "matches_expected" "true"

bundle_imported_task_output="$(run_workvcs \
    task show "$bundle_target_store" \
    --branch "$bundle_branch_id" \
    --task "$bundle_task_id")"
expect_value "$bundle_imported_task_output" "task_entity_id" "$bundle_task_id"
expect_value "$bundle_imported_task_output" "status" "pending"
expect_value "$bundle_imported_task_output" "priority" "5"

bundle_target_checkpoint_latest_output="$(run_workvcs \
    checkpoint latest "$bundle_target_store" \
    --commit "$bundle_head_id" \
    --require-found \
    --expected-checkpoint "$bundle_checkpoint_id")"
expect_value "$bundle_target_checkpoint_latest_output" "commit_id" "$bundle_head_id"
expect_value "$bundle_target_checkpoint_latest_output" "checkpoint_found" "true"
expect_value "$bundle_target_checkpoint_latest_output" "checkpoint_id" "$bundle_checkpoint_id"
expect_value "$bundle_target_checkpoint_latest_output" "checkpoint_found_required" "true"
expect_value "$bundle_target_checkpoint_latest_output" "checkpoint_matches_expected" "true"

bundle_target_checkpoint_show_output="$(run_workvcs \
    checkpoint show "$bundle_target_store" \
    --checkpoint "$bundle_checkpoint_id" \
    --expected-state-digest "$bundle_state_digest" \
    --expected-content-digest "$bundle_checkpoint_content_digest")"
expect_value "$bundle_target_checkpoint_show_output" "checkpoint_id" "$bundle_checkpoint_id"
expect_value "$bundle_target_checkpoint_show_output" "commit_id" "$bundle_head_id"
expect_value "$bundle_target_checkpoint_show_output" "state_matches_expected" "true"
expect_value "$bundle_target_checkpoint_show_output" "content_matches_expected" "true"
expect_value "$bundle_target_checkpoint_show_output" "usability_state" "usable"

bundle_target_checkpoint_validate_output="$(run_workvcs \
    checkpoint validate "$bundle_target_store" \
    --checkpoint "$bundle_checkpoint_id" \
    --require-valid)"
expect_value "$bundle_target_checkpoint_validate_output" "checkpoint_id" "$bundle_checkpoint_id"
expect_value "$bundle_target_checkpoint_validate_output" "valid" "true"
expect_value "$bundle_target_checkpoint_validate_output" "problem" "none"
expect_value "$bundle_target_checkpoint_validate_output" "expected_state_digest" "$bundle_state_digest"
expect_value "$bundle_target_checkpoint_validate_output" "expected_content_digest" "$bundle_checkpoint_content_digest"
expect_value "$bundle_target_checkpoint_validate_output" "valid_required" "true"

bundle_import_show_output="$(run_workvcs \
    bundle import-show "$bundle_target_store" \
    --import "$bundle_import_id" \
    --expected-bundle-digest "$bundle_digest" \
    --expected-outcome same_store_fast_forward_applied)"
expect_value "$bundle_import_show_output" "import_id" "$bundle_import_id"
expect_value "$bundle_import_show_output" "outcome" "same_store_fast_forward_applied"
expect_value "$bundle_import_show_output" "bundle_matches_expected" "true"
expect_value "$bundle_import_show_output" "outcome_matches_expected" "true"

bundle_import_list_output="$(run_workvcs \
    bundle import-list "$bundle_target_store" \
    --bundle-digest "$bundle_digest" \
    --outcome same_store_fast_forward_applied \
    --expected-imports 1)"
expect_value "$bundle_import_list_output" "imports" "1"
expect_value "$bundle_import_list_output" "imports_match_expected" "true"
expect_value "$bundle_import_list_output" "import[0].import_id" "$bundle_import_id"
expect_value "$bundle_import_list_output" "import[0].outcome" "same_store_fast_forward_applied"

step "runtime closeout"
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
    --expected-checked-branches 2 \
    --expected-checked-commits 13 \
    --expected-checked-changesets 13 \
    --expected-checked-change-operations 17 \
    --expected-checked-changeset-causal-anchors 0 \
    --expected-checked-events 24 \
    --expected-checked-checkpoints 1 \
    --expected-invalid-checkpoints 0)"
expect_value "$store_integrity_output" "checked_branches" "2"
expect_value "$store_integrity_output" "checked_commits" "13"
expect_value "$store_integrity_output" "checked_changesets" "13"
expect_value "$store_integrity_output" "checked_change_operations" "17"
expect_value "$store_integrity_output" "checked_changeset_causal_anchors" "0"
expect_value "$store_integrity_output" "checked_events" "24"
expect_value "$store_integrity_output" "checked_checkpoints" "1"
expect_value "$store_integrity_output" "invalid_checkpoints" "0"
expect_integrity_matches "$store_integrity_output"

doctor_output="$(run_workvcs \
    doctor "$store" \
    --require-valid \
    --expected-checked-branches 2 \
    --expected-checked-commits 13 \
    --expected-checked-changesets 13 \
    --expected-checked-change-operations 17 \
    --expected-checked-changeset-causal-anchors 0 \
    --expected-checked-events 24 \
    --expected-checked-checkpoints 1 \
    --expected-invalid-checkpoints 0)"
expect_contains "$doctor_output" "ok store_id=$store_id schema_version=1 canonical_json_profile=workvcs-jcs-v1"
expect_contains "$doctor_output" "checked_branches=2 checked_commits=13 checked_changesets=13 checked_change_operations=17 checked_changeset_causal_anchors=0 checked_events=24 checked_checkpoints=1 invalid_checkpoints=0"
expect_integrity_matches "$doctor_output"

printf 'smoke_result=passed\n'
printf 'store_id=%s\n' "$store_id"
printf 'workspace_id=%s\n' "$workspace_id"
printf 'branch_id=%s\n' "$branch_id"
printf 'genesis_state_digest=%s\n' "$genesis_state_digest"
printf 'task_entity_id=%s\n' "$task_id"
printf 'verification_entity_id=%s\n' "$verification_id"
printf 'claim_id=%s\n' "$claim_id"
printf 'source_branch_id=%s\n' "$source_branch_id"
printf 'merge_id=%s\n' "$merge_id"
printf 'merge_result_commit_id=%s\n' "$result_commit_id"
printf 'checkpoint_id=%s\n' "$checkpoint_id"
printf 'bundle_checkpoint_id=%s\n' "$bundle_checkpoint_id"
printf 'bundle_import_id=%s\n' "$bundle_import_id"
printf 'session_diff_id=%s\n' "$session_diff_id"
