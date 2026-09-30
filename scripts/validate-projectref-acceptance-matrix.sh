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

[[ "$(wc -l <"$tmp_dir/contract-ids" | tr -d ' ')" == "93" ]] || {
    echo "acceptance contract does not contain exactly 93 matrix rows" >&2
    exit 1
}
[[ "$(wc -l <"$tmp_dir/ledger-ids" | tr -d ' ')" == "93" ]] || {
    echo "acceptance ledger does not contain exactly 93 matrix rows" >&2
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

if grep -R -i -n -E 'codex|chatgpt' "$repo_root/crates/workvcs-core/src/control_plane"; then
    echo "tool-specific term leaked into the generic core control plane" >&2
    exit 1
fi

echo "matrix_rows=93"
echo "matrix_ids_exact=true"
echo "matrix_statuses_valid=true"
echo "round5_value_gate_and_no_record_policy_probe=passed"
echo "round5_durability_claim_wording_probe=passed"
echo "required_participation_policy_probe=passed"
echo "activation_refresh_contract_probe=passed"
echo "control_plane_selection_fallback_probe=passed"
echo "adapter_docs_aligned=true"
echo "generic_core_tool_neutral=true"
