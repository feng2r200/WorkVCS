#!/usr/bin/env bash
set -euo pipefail

script_name="$(basename "$0")"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"

cargo_bin="${CARGO:-cargo}"
rustc_bin="${RUSTC:-rustc}"

profile="release"
target_triple=""
package_dir="$repo_root/target/package"
bin_dir="/usr/local/bin"
dest_path=""
install_requested="0"
dry_run="0"
use_locked="1"

usage() {
    cat <<EOF
Usage: $script_name [options]

Build and package the current checkout's workvcs binary. By default this script
only creates a local package artifact; it never installs into a system path
unless --install is provided.

Options:
  --install             Install or overwrite workvcs after packaging.
  --dry-run             Print planned build/package/install actions only.
  --bin-dir DIR         Install directory for --install (default: /usr/local/bin).
  --prefix DIR          Install under DIR/bin for --install.
  --dest PATH           Exact install destination; basename must be workvcs.
  --package-dir DIR     Artifact output directory (default: target/package).
  --profile PROFILE     Cargo profile: release or debug (default: release).
  --target TRIPLE       Optional path-safe cargo target triple.
  --no-locked           Do not pass --locked to cargo build.
  -h, --help            Show this help.

Examples:
  scripts/package-workvcs.sh
  scripts/package-workvcs.sh --dry-run --install
  scripts/package-workvcs.sh --install --bin-dir /tmp/workvcs-bin
  scripts/package-workvcs.sh --install --bin-dir /usr/local/bin
EOF
}

die() {
    printf 'workvcs_package_error=%s\n' "$*" >&2
    exit 1
}

step() {
    printf 'workvcs_package_step=%s\n' "$*" >&2
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
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

is_same_target_as_host() {
    local requested="$1"
    local host="$2"
    [[ -z "$requested" || "$requested" == "$host" ]]
}

ensure_dest_basename() {
    local path="$1"
    local name
    name="$(basename "$path")"
    [[ "$name" == "workvcs" ]] || die "install destination basename must be workvcs"
}

ensure_target_triple_segment() {
    local value="$1"
    [[ -n "$value" ]] || die "--target requires a non-empty target triple"
    [[ "$value" != *"/"* ]] || die "--target must be a target triple, not a path"
    [[ "$value" != *"\\"* ]] || die "--target must be a target triple, not a path"
    [[ "$value" != *".."* ]] || die "--target must not contain '..'"
    [[ "$value" =~ ^[A-Za-z0-9._+-]+$ ]] || die "--target contains unsupported characters"
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --install)
            install_requested="1"
            ;;
        --dry-run)
            dry_run="1"
            ;;
        --bin-dir)
            [[ "$#" -ge 2 ]] || die "--bin-dir requires a value"
            bin_dir="$(absolute_path "$2")"
            shift
            ;;
        --prefix)
            [[ "$#" -ge 2 ]] || die "--prefix requires a value"
            bin_dir="$(absolute_path "$2")/bin"
            shift
            ;;
        --dest)
            [[ "$#" -ge 2 ]] || die "--dest requires a value"
            dest_path="$(absolute_path "$2")"
            shift
            ;;
        --package-dir)
            [[ "$#" -ge 2 ]] || die "--package-dir requires a value"
            package_dir="$(absolute_path "$2")"
            shift
            ;;
        --profile)
            [[ "$#" -ge 2 ]] || die "--profile requires a value"
            profile="$2"
            shift
            ;;
        --target)
            [[ "$#" -ge 2 ]] || die "--target requires a value"
            target_triple="$2"
            shift
            ;;
        --no-locked)
            use_locked="0"
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

case "$profile" in
    release|debug) ;;
    *) die "--profile must be release or debug" ;;
esac
if [[ -n "$target_triple" ]]; then
    ensure_target_triple_segment "$target_triple"
fi

if [[ -n "$dest_path" ]]; then
    install_path="$dest_path"
else
    install_path="$bin_dir/workvcs"
fi
ensure_dest_basename "$install_path"

if [[ "$dry_run" != "1" ]]; then
    require_command "$cargo_bin"
    require_command "$rustc_bin"
    require_command tar
    [[ -f "$repo_root/crates/workvcs-cli/Cargo.toml" ]] || die "missing workvcs-cli Cargo.toml"
fi

host_triple="unknown-host"
if command -v "$rustc_bin" >/dev/null 2>&1; then
    host_triple="$("$rustc_bin" -vV | awk '/^host:/ {print $2}')"
fi
artifact_target="${target_triple:-$host_triple}"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
git_head="$(git -C "$repo_root" rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown')"
git_head_full="$(git -C "$repo_root" rev-parse HEAD 2>/dev/null || printf 'unknown')"
git_dirty="unknown"
if git -C "$repo_root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    if [[ -n "$(git -C "$repo_root" status --porcelain)" ]]; then
        git_dirty="true"
    else
        git_dirty="false"
    fi
fi
artifact_name="workvcs-${artifact_target}-${git_head}-${timestamp}"
artifact_root="$package_dir/$artifact_name"
package_binary="$artifact_root/bin/workvcs"
manifest_path="$artifact_root/manifest.txt"
archive_path="$package_dir/$artifact_name.tar.gz"

cargo_args=(build --package workvcs-cli --bin workvcs)
if [[ "$use_locked" == "1" && -f "$repo_root/Cargo.lock" ]]; then
    cargo_args+=(--locked)
fi
if [[ "$profile" == "release" ]]; then
    cargo_args+=(--release)
fi
if [[ -n "$target_triple" ]]; then
    cargo_args+=(--target "$target_triple")
fi

profile_dir="$profile"
if [[ "$profile" == "debug" ]]; then
    profile_dir="debug"
fi
if [[ -n "$target_triple" ]]; then
    built_binary="$repo_root/target/$target_triple/$profile_dir/workvcs"
else
    built_binary="$repo_root/target/$profile_dir/workvcs"
fi

if [[ "$install_requested" == "1" ]] && ! is_same_target_as_host "$target_triple" "$host_triple"; then
    die "refusing to install cross-target binary; package only or rerun without --target"
fi

if [[ "$dry_run" == "1" ]]; then
    printf 'workvcs_package_dry_run=true\n'
    printf 'workvcs_package_repo_root=%s\n' "$repo_root"
    printf 'workvcs_package_would_build='
    printf '%q ' "$cargo_bin" "${cargo_args[@]}"
    printf '\n'
    printf 'workvcs_package_would_create_dir=%s\n' "$artifact_root"
    printf 'workvcs_package_would_archive=%s\n' "$archive_path"
    if [[ "$install_requested" == "1" ]]; then
        printf 'workvcs_install_would_write=%s\n' "$install_path"
        if [[ -e "$install_path" ]]; then
            printf 'workvcs_install_would_overwrite=true\n'
        else
            printf 'workvcs_install_would_overwrite=false\n'
        fi
        printf 'workvcs_install_system_path=%s\n' "$([[ "$install_path" == /usr/local/bin/workvcs ]] && printf true || printf false)"
    else
        printf 'workvcs_install_requested=false\n'
    fi
    exit 0
fi

step "build workvcs"
(cd "$repo_root" && "$cargo_bin" "${cargo_args[@]}")
[[ -x "$built_binary" ]] || die "missing built workvcs binary at $built_binary"

step "create package artifact"
mkdir -p "$artifact_root/bin"
install -m 0755 "$built_binary" "$package_binary"

binary_sha256="$(file_sha256 "$package_binary")"
binary_size="$(wc -c < "$package_binary" | tr -d ' ')"

cat > "$manifest_path" <<EOF
name=workvcs
cargo_package=workvcs-cli
cargo_bin=workvcs
source_repo=$repo_root
source_git_head=$git_head_full
source_git_dirty=$git_dirty
profile=$profile
target=$artifact_target
built_at_utc=$timestamp
binary_path=bin/workvcs
binary_sha256=$binary_sha256
binary_size_bytes=$binary_size
default_install_path=/usr/local/bin/workvcs
install_mode=explicit_only
EOF

step "archive package"
mkdir -p "$package_dir"
tar -C "$package_dir" -czf "$archive_path" "$artifact_name"

if is_same_target_as_host "$target_triple" "$host_triple"; then
    step "validate packaged binary"
    "$package_binary" --help >/dev/null
    package_binary_verified="true"
else
    package_binary_verified="skipped_cross_target"
fi

printf 'workvcs_package_dir=%s\n' "$artifact_root"
printf 'workvcs_package_archive=%s\n' "$archive_path"
printf 'workvcs_package_manifest=%s\n' "$manifest_path"
printf 'workvcs_package_binary=%s\n' "$package_binary"
printf 'workvcs_package_binary_sha256=%s\n' "$binary_sha256"
printf 'workvcs_package_binary_verified=%s\n' "$package_binary_verified"

install_atomically() {
    local source_path="$1"
    local final_path="$2"
    local final_dir
    local temp_path
    local sudo_bin

    final_dir="$(dirname "$final_path")"
    if [[ ! -d "$final_dir" ]]; then
        if mkdir -p "$final_dir" 2>/dev/null; then
            :
        else
            require_command sudo
            sudo mkdir -p "$final_dir"
        fi
    fi

    if [[ -w "$final_dir" ]]; then
        temp_path="$(mktemp "$final_dir/.workvcs.tmp.XXXXXX")"
        install -m 0755 "$source_path" "$temp_path"
        mv -f "$temp_path" "$final_path"
    else
        require_command sudo
        sudo_bin="$(command -v sudo)"
        temp_path="$final_dir/.workvcs.tmp.$$"
        "$sudo_bin" install -m 0755 "$source_path" "$temp_path"
        "$sudo_bin" mv -f "$temp_path" "$final_path"
    fi
}

if [[ "$install_requested" == "1" ]]; then
    step "install workvcs"
    install_atomically "$package_binary" "$install_path"
    [[ -x "$install_path" ]] || die "installed path is not executable: $install_path"
    "$install_path" --help >/dev/null
    installed_sha256="$(file_sha256 "$install_path")"
    [[ "$installed_sha256" == "$binary_sha256" ]] || die "installed binary digest mismatch"
    printf 'workvcs_install_path=%s\n' "$install_path"
    printf 'workvcs_install_sha256=%s\n' "$installed_sha256"
    printf 'workvcs_install_verified=true\n'
fi
