#!/usr/bin/env bash
set -euo pipefail

script_name="$(basename "$0")"
roots=()
keep_artifacts=()
dry_run="0"

usage() {
    cat <<EOF
Usage: $script_name --root DIR [--root DIR ...] [options]

Remove verified historical WorkVCS package artifacts from explicitly named
roots. The script only removes direct workvcs-* package directories and their
matching archives whose manifest and binary digest are valid. Unrecognized or
tampered entries are preserved and reported.

Options:
  --root DIR             Package root to inspect; repeat for each exact root.
  --keep-artifact DIR    Exact package directory to preserve; repeat as needed.
  --dry-run              Report removals without changing files.
  -h, --help             Show this help.
EOF
}

die() {
    printf 'workvcs_prune_error=%s\n' "$*" >&2
    exit 1
}

absolute_path() {
    local path="$1"
    case "$path" in
        /*) printf '%s\n' "$path" ;;
        *) printf '%s/%s\n' "$(pwd -P)" "$path" ;;
    esac
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

archive_binary_sha256() {
    local archive="$1"
    local archive_name="$2"
    tar -xOf "$archive" "$archive_name/bin/workvcs" 2>/dev/null \
        | file_sha256 /dev/stdin
}

manifest_value() {
    local manifest="$1"
    local key="$2"
    awk -F= -v wanted="$key" '$1 == wanted { print substr($0, index($0, "=") + 1); found=1; exit } END { if (!found) exit 1 }' "$manifest"
}

is_kept_artifact() {
    local candidate="$1"
    local kept
    for kept in "${keep_artifacts[@]}"; do
        if [[ "$candidate" == "$kept" ]]; then
            return 0
        fi
    done
    return 1
}

remove_path() {
    local path="$1"
    if [[ "$dry_run" == "1" ]]; then
        printf 'workvcs_prune_would_remove=%s\n' "$path"
    else
        rm -rf -- "$path"
        printf 'workvcs_prune_removed=%s\n' "$path"
    fi
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --root)
            [[ "$#" -ge 2 ]] || die "--root requires a value"
            roots+=("$(absolute_path "$2")")
            shift
            ;;
        --keep-artifact)
            [[ "$#" -ge 2 ]] || die "--keep-artifact requires a value"
            keep_artifacts+=("$(absolute_path "$2")")
            shift
            ;;
        --dry-run)
            dry_run="1"
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

[[ "${#roots[@]}" -gt 0 ]] || die "at least one --root is required"
command -v tar >/dev/null 2>&1 || die "missing required command: tar"

removed="0"
skipped="0"
for root in "${roots[@]}"; do
    [[ -d "$root" ]] || die "package root does not exist: $root"
    root="$(cd "$root" && pwd -P)"
    for candidate in "$root"/workvcs-*; do
        [[ -d "$candidate" ]] || continue
        if [[ -L "$candidate" ]]; then
            printf 'workvcs_prune_skipped=%s\n' "$candidate"
            skipped=$((skipped + 1))
            continue
        fi
        candidate="$(cd "$candidate" && pwd -P)"
        manifest="$candidate/manifest.txt"
        binary="$candidate/bin/workvcs"
        if [[ ! -f "$manifest" || -L "$manifest" || ! -f "$binary" || -L "$binary" ]]; then
            printf 'workvcs_prune_skipped=%s\n' "$candidate"
            skipped=$((skipped + 1))
            continue
        fi
        if [[ "$(manifest_value "$manifest" name 2>/dev/null || true)" != "workvcs" \
            || "$(manifest_value "$manifest" cargo_bin 2>/dev/null || true)" != "workvcs" \
            || "$(manifest_value "$manifest" binary_path 2>/dev/null || true)" != "bin/workvcs" ]]; then
            printf 'workvcs_prune_skipped=%s\n' "$candidate"
            skipped=$((skipped + 1))
            continue
        fi
        expected_sha="$(manifest_value "$manifest" binary_sha256 2>/dev/null || true)"
        actual_sha="$(file_sha256 "$binary")"
        if [[ -z "$expected_sha" || "$actual_sha" != "$expected_sha" ]]; then
            printf 'workvcs_prune_skipped=%s\n' "$candidate"
            printf 'workvcs_prune_skip_reason=binary_manifest_digest_mismatch\n'
            skipped=$((skipped + 1))
            continue
        fi
        if is_kept_artifact "$candidate"; then
            printf 'workvcs_prune_kept=%s\n' "$candidate"
            continue
        fi
        remove_path "$candidate"
        removed=$((removed + 1))
        archive="$root/$(basename "$candidate").tar.gz"
        if [[ -f "$archive" && ! -L "$archive" ]]; then
            remove_path "$archive"
            removed=$((removed + 1))
        fi
    done

    for archive in "$root"/workvcs-*.tar.gz; do
        [[ -f "$archive" && ! -L "$archive" ]] || continue
        archive_name="$(basename "$archive" .tar.gz)"
        archive_manifest=""
        if ! archive_manifest="$(tar -xOf "$archive" "$archive_name/manifest.txt" 2>/dev/null)"; then
            printf 'workvcs_prune_skipped=%s\n' "$archive"
            skipped=$((skipped + 1))
            continue
        fi
        if ! printf '%s\n' "$archive_manifest" | awk -F= '
            $1 == "name" && $2 == "workvcs" { name=1 }
            $1 == "cargo_bin" && $2 == "workvcs" { bin=1 }
            $1 == "binary_path" && $2 == "bin/workvcs" { path=1 }
            END { exit !(name && bin && path) }
        '; then
            printf 'workvcs_prune_skipped=%s\n' "$archive"
            skipped=$((skipped + 1))
            continue
        fi
        expected_sha="$(printf '%s\n' "$archive_manifest" \
            | awk -F= '$1 == "binary_sha256" { print $2; exit }')"
        actual_sha=""
        if [[ -n "$expected_sha" ]]; then
            actual_sha="$(archive_binary_sha256 "$archive" "$archive_name" || true)"
        fi
        if [[ -z "$expected_sha" || "$actual_sha" != "$expected_sha" ]]; then
            printf 'workvcs_prune_skipped=%s\n' "$archive"
            printf 'workvcs_prune_skip_reason=archive_binary_manifest_digest_mismatch\n'
            skipped=$((skipped + 1))
            continue
        fi
        candidate="$root/$archive_name"
        if is_kept_artifact "$candidate"; then
            printf 'workvcs_prune_kept=%s\n' "$archive"
            continue
        fi
        [[ -e "$candidate" ]] && continue
        remove_path "$archive"
        removed=$((removed + 1))
    done
done

printf 'workvcs_prune_dry_run=%s\n' "$([[ "$dry_run" == "1" ]] && printf true || printf false)"
printf 'workvcs_prune_removed_count=%s\n' "$removed"
printf 'workvcs_prune_skipped_count=%s\n' "$skipped"
