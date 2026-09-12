#!/usr/bin/env bash
set -euo pipefail

script_name="$(basename "$0")"

usage() {
    cat <<EOF
Usage: $script_name create SKILL_DIR MANIFEST
       $script_name verify SKILL_DIR MANIFEST

Create or verify a deterministic SHA-256 manifest for every regular file in a
WorkVCS Skill tree. MANIFEST must live outside SKILL_DIR.
EOF
}

die() {
    printf 'workvcs_skill_tree_error=%s\n' "$*" >&2
    exit 1
}

file_sha256() {
    local path="$1"
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$path" | awk '{print $1}'
    elif command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" | awk '{print $1}'
    else
        die "missing sha256 command: shasum or sha256sum"
    fi
}

path_has_control_character() {
    local path="$1"
    LC_ALL=C printf '%s' "$path" | LC_ALL=C grep -q '[[:cntrl:]]'
}

write_manifest() (
    local skill_dir="$1"
    local manifest="$2"
    local manifest_dir
    local temp_manifest
    local temp_paths
    local sorted_paths
    local relative_path
    local digest

    manifest_dir="$(dirname "$manifest")"
    [[ -d "$manifest_dir" ]] || die "manifest directory does not exist: $manifest_dir"
    temp_manifest="$(mktemp "$manifest_dir/.workvcs-skill-tree.tmp.XXXXXX")"
    temp_paths="$(mktemp "${TMPDIR:-/tmp}/workvcs-skill-tree.paths.XXXXXX")"
    sorted_paths="$(mktemp "${TMPDIR:-/tmp}/workvcs-skill-tree.sorted.XXXXXX")"
    trap 'rm -f "$temp_manifest" "$temp_paths" "$sorted_paths"' EXIT

    (
        cd "$skill_dir"
        LC_ALL=C find . -type f -print0 > "$temp_paths"
    ) || die "failed to enumerate Skill tree: $skill_dir"

    while IFS= read -r -d '' relative_path; do
        relative_path="${relative_path#./}"
        if path_has_control_character "$relative_path"; then
            die "Skill path contains an unsupported control character"
        fi
        printf '%s\n' "$relative_path" >> "$sorted_paths"
    done < "$temp_paths"
    LC_ALL=C sort -o "$sorted_paths" "$sorted_paths" || die "failed to sort Skill paths"

    while IFS= read -r relative_path; do
        digest="$(file_sha256 "$skill_dir/$relative_path")" || \
            die "failed to hash Skill file: $relative_path"
        [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || \
            die "invalid SHA-256 digest for Skill file: $relative_path"
        printf '%s  %s\n' "$digest" "$relative_path" >> "$temp_manifest"
    done < "$sorted_paths"

    mv -f "$temp_manifest" "$manifest"
    trap - EXIT
    rm -f "$temp_paths" "$sorted_paths"
)

report_manifest() {
    local manifest="$1"
    local file_count
    local digest
    file_count="$(wc -l < "$manifest" | tr -d ' ')"
    digest="$(file_sha256 "$manifest")" || die "failed to hash Skill tree manifest"
    [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || die "invalid Skill tree manifest SHA-256"
    printf 'workvcs_skill_tree_files=%s\n' "$file_count"
    printf 'workvcs_skill_tree_manifest_sha256=%s\n' "$digest"
}

reject_unsupported_entries() {
    local skill_dir="$1"
    local unsupported
    unsupported="$(
        cd "$skill_dir"
        find . ! -type d ! -type f -print -quit
    )"
    [[ -z "$unsupported" ]] || die "Skill tree contains unsupported entry: ${unsupported#./}"
}

[[ "$#" -eq 3 ]] || {
    usage >&2
    exit 2
}

mode="$1"
skill_dir="$2"
manifest="$3"

[[ -d "$skill_dir" ]] || die "Skill directory does not exist: $skill_dir"
[[ -f "$skill_dir/SKILL.md" ]] || die "Skill entrypoint is missing: $skill_dir/SKILL.md"
reject_unsupported_entries "$skill_dir"

case "$mode" in
    create)
        write_manifest "$skill_dir" "$manifest"
        printf 'workvcs_skill_tree_created=true\n'
        report_manifest "$manifest"
        ;;
    verify)
        [[ -f "$manifest" ]] || die "Skill tree manifest does not exist: $manifest"
        actual_manifest="$(mktemp "${TMPDIR:-/tmp}/workvcs-skill-tree.actual.XXXXXX")"
        trap 'rm -f "$actual_manifest"' EXIT
        write_manifest "$skill_dir" "$actual_manifest"
        if ! cmp -s "$manifest" "$actual_manifest"; then
            diff -u "$manifest" "$actual_manifest" >&2 || true
            die "Skill tree does not match manifest"
        fi
        printf 'workvcs_skill_tree_verified=true\n'
        report_manifest "$actual_manifest"
        ;;
    *)
        usage >&2
        die "mode must be create or verify"
        ;;
esac
