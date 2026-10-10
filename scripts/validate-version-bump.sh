#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"
base_ref="${VERSION_BASE_REF:-HEAD^}"

usage() {
    cat <<'EOF'
Usage: scripts/validate-version-bump.sh [--base-ref REF]

Require a higher WorkVCS semantic version when the current change touches the
CLI, core, Skill, packaging, installation, or release workflow.
EOF
}

die() {
    printf 'workvcs_version_gate_error=%s\n' "$*" >&2
    exit 1
}

read_workspace_version() {
    awk -F'"' '
        /^\[workspace\.package\]$/ { in_workspace=1; next }
        /^\[/ { in_workspace=0 }
        in_workspace && /^version[[:space:]]*=[[:space:]]*"/ { print $2; exit }
    '
}

read_version_at_ref() {
    local ref="$1"
    local manifest
    manifest="$(git -C "$repo_root" show "$ref:Cargo.toml" 2>/dev/null)" \
        || die "cannot read Cargo.toml at base ref $ref"
    local version
    version="$(printf '%s\n' "$manifest" | read_workspace_version)"
    if [[ -n "$version" ]]; then
        printf '%s\n' "$version"
        return
    fi
    git -C "$repo_root" show "$ref:crates/workvcs-cli/Cargo.toml" 2>/dev/null \
        | awk -F'"' '/^version[[:space:]]*=[[:space:]]*"/ { print $2; exit }'
}

validate_version() {
    local version="$1"
    [[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] \
        || die "invalid semantic version: $version"
}

version_greater_than() {
    local current="$1"
    local base="$2"
    local current_major current_minor current_patch
    local base_major base_minor base_patch
    IFS=. read -r current_major current_minor current_patch <<< "$current"
    IFS=. read -r base_major base_minor base_patch <<< "$base"
    if (( current_major > base_major )); then
        return 0
    fi
    if (( current_major < base_major )); then
        return 1
    fi
    if (( current_minor > base_minor )); then
        return 0
    fi
    if (( current_minor < base_minor )); then
        return 1
    fi
    (( current_patch > base_patch ))
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --base-ref)
            [[ "$#" -ge 2 ]] || die "--base-ref requires a value"
            base_ref="$2"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "unknown option: $1"
            ;;
    esac
    shift
done

git -C "$repo_root" rev-parse --verify "$base_ref^{commit}" >/dev/null \
    || die "base ref does not resolve to a commit: $base_ref"
current_version="$(read_workspace_version < "$repo_root/Cargo.toml")"
base_version="$(read_version_at_ref "$base_ref")"
[[ -n "$current_version" && -n "$base_version" ]] \
    || die "could not determine current and base WorkVCS versions"
validate_version "$current_version"
validate_version "$base_version"

release_change="false"
while IFS= read -r changed_file; do
    case "$changed_file" in
        Cargo.toml|Cargo.lock|crates/*|scripts/*|skills/*|.github/workflows/*)
            release_change="true"
            break
            ;;
    esac
done < <(git -C "$repo_root" diff --name-only "$base_ref")

if [[ "$release_change" == "true" && "$current_version" == "$base_version" ]]; then
    die "release-impacting changes require a semantic version bump (still $current_version)"
fi
if [[ "$current_version" != "$base_version" ]] && ! version_greater_than "$current_version" "$base_version"; then
    die "current version $current_version must be greater than base version $base_version"
fi

printf 'workvcs_version_base=%s\n' "$base_version"
printf 'workvcs_version_current=%s\n' "$current_version"
printf 'workvcs_release_change=%s\n' "$release_change"
printf 'workvcs_version_bump_verified=%s\n' "$([[ "$current_version" != "$base_version" ]] && printf true || printf false)"
