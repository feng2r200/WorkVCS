#!/usr/bin/env bash
set -euo pipefail

# Explicit, per-user macOS adoption. The ordinary package installer never calls
# this script; rendering and installing a plist do not start a background job.
action=status
action_selected=false
binary="${HOME:?HOME is required}/.local/bin/workvcs"
registry=""
root=""
agent_dir="${HOME}/Library/LaunchAgents"
periodic_seconds=900
label=io.workvcs.automation

die() { printf 'workvcs_launchagent_error=%s\n' "$*" >&2; exit 1; }
usage() {
    cat <<'EOF'
Usage: scripts/workvcs-launchagent.sh ACTION --registry ABSOLUTE_PATH [options]

Actions (choose one; default --status):
  --render    Print the exact plist; no filesystem or launchd changes.
  --install   Install the plist without loading it; identical replay is no-op.
  --start     Enable and load the already installed exact plist.
  --status    Read installed plist and launchd status.
  --stop      Unload this login session's job; retain the login startup plist.
  --disable   Disable and unload the job; retain the plist for recovery.

Options:
  --binary PATH       Stable WorkVCS executable (default: $HOME/.local/bin/workvcs).
  --registry PATH     Explicit existing ProjectRef registry (required).
  --root PATH         Derived automation root (default: REGISTRY.d/automation/v1).
  --agent-dir PATH    LaunchAgents directory (default: $HOME/Library/LaunchAgents).
  --periodic-seconds N  Anti-entropy interval (default: 900).

The service runs as the current user, uses no TCP port, and retains its private
logs under ROOT/runtime/launchagent. Existing different or symlinked plists are
refused. Installation and startup do not authorize new rollout roots.
EOF
}
select_action() {
    [[ "$action_selected" == false ]] || die "choose exactly one action"
    action="$1"
    action_selected=true
}
while [[ $# -gt 0 ]]; do
    case "$1" in
        --render|--install|--start|--status|--stop|--disable) select_action "${1#--}" ;;
        --binary|--registry|--root|--agent-dir|--periodic-seconds)
            [[ $# -ge 2 ]] || die "$1 requires a value"
            case "$1" in
                --binary) binary="$2" ;;
                --registry) registry="$2" ;;
                --root) root="$2" ;;
                --agent-dir) agent_dir="$2" ;;
                --periodic-seconds) periodic_seconds="$2" ;;
            esac
            shift ;;
        -h|--help) usage; exit 0 ;;
        *) die "unknown option: $1" ;;
    esac
    shift
done
[[ -n "$registry" ]] || die "--registry is required"
[[ -n "$root" ]] || root="${registry}.d/automation/v1"
[[ "$periodic_seconds" =~ ^[1-9][0-9]*$ ]] || die "periodic interval must be a positive integer"
for path in "$binary" "$registry" "$root" "$agent_dir"; do
    [[ "$path" == /* && "$path" != / && "$path" != "$HOME" ]] || die "paths must be scoped absolute paths"
    [[ "$path" != *$'\n'* && "$path" != *$'\r'* ]] || die "paths must not contain line breaks"
    [[ ! -L "$path" ]] || die "refusing symlink: $path"
done
plist="$agent_dir/$label.plist"
log_dir="$root/runtime/launchagent"
domain="gui/$(id -u)/$label"

xml_escape() {
    printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
}
render() {
    cat <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>$label</string>
<key>ProgramArguments</key><array>
<string>$(xml_escape "$binary")</string><string>automation</string><string>daemon</string><string>run</string>
<string>--registry</string><string>$(xml_escape "$registry")</string>
<string>--root</string><string>$(xml_escape "$root")</string>
<string>--periodic-seconds</string><string>$periodic_seconds</string>
</array>
<key>RunAtLoad</key><true/>
<key>KeepAlive</key><true/>
<key>ThrottleInterval</key><integer>10</integer>
<key>Umask</key><integer>63</integer>
<key>WorkingDirectory</key><string>$(xml_escape "$root")</string>
<key>StandardOutPath</key><string>$(xml_escape "$log_dir/stdout.log")</string>
<key>StandardErrorPath</key><string>$(xml_escape "$log_dir/stderr.log")</string>
</dict></plist>
EOF
}
if [[ "$action" == render ]]; then render; exit 0; fi
[[ "$(uname -s)" == Darwin ]] || die "launchd adoption requires macOS; --render is portable"
check_directory() {
    local directory="$1" private="$2" mode mask=022
    [[ -d "$directory" && -O "$directory" && ! -L "$directory" ]] || die "unsafe current-user directory: $directory"
    [[ "$private" != true ]] || mask=077
    mode="$(stat -f '%Lp' "$directory")"
    (( (8#$mode & mask) == 0 )) || die "unsafe directory permissions: $directory"
}
[[ ! -e "$agent_dir" ]] || check_directory "$agent_dir" false
for directory in "$root" "$root/runtime" "$log_dir"; do
    [[ ! -e "$directory" ]] || check_directory "$directory" true
done
[[ ! -L "$plist" ]] || die "refusing symlinked plist: $plist"
if [[ -e "$plist" ]]; then
    [[ -f "$plist" && -O "$plist" ]] || die "plist is not a current-user regular file"
    plist_mode="$(stat -f '%Lp' "$plist")"
    (( (8#$plist_mode & 022) == 0 )) || die "plist is writable by another user; preserve it for review"
    cmp -s "$plist" <(render) || die "existing plist differs; preserve it and review the explicit replacement"
fi

stable_gate() {
    [[ -x "$binary" && -f "$registry" && ! -L "$registry" ]] || die "stable executable or registry unavailable"
    [[ -O "$binary" || "$(stat -f '%u' "$binary")" == 0 ]] || die "executable is not owned by current user or root"
    [[ -O "$registry" ]] || die "registry is not owned by current user"
    local file file_mode
    for file in "$binary" "$registry"; do
        file_mode="$(stat -f '%Lp' "$file")"
        (( (8#$file_mode & 022) == 0 )) || die "executable or registry is writable by another user"
    done
    local runtime
    runtime="$("$binary" runtime status --require-stable)" || die "stable runtime gate failed"
    runtime+=$'\n'
    [[ "$runtime" == *$'\nexecutable_role=stable_installed\n'* ]] || die "not the stable installed runtime"
    [[ "$runtime" == *$'\nbuild_dirty=false\n'* ]] || die "installed runtime was built from dirty source"
}
private_directory() {
    local directory="$1"
    [[ ! -L "$directory" ]] || die "refusing symlinked directory: $directory"
    if [[ ! -e "$directory" ]]; then (umask 077; mkdir -p "$directory"); fi
    [[ -d "$directory" && -O "$directory" ]] || die "directory is not owned by current user: $directory"
    local mode
    mode="$(stat -f '%Lp' "$directory")"
    (( (8#$mode & 077) == 0 )) || die "derived directory is not owner-private: $directory"
}
loaded_job_matches() {
    local actual_path actual_program actual_arguments expected_arguments
    actual_path="$(printf '%s\n' "$1" | sed -n 's/^[[:space:]]*path = //p' | head -1)"
    actual_program="$(printf '%s\n' "$1" | sed -n 's/^[[:space:]]*program = //p' | head -1)"
    actual_arguments="$(printf '%s\n' "$1" | awk '
        /^[[:space:]]*arguments = \{/ {reading=1; next}
        reading && /^[[:space:]]*\}/ {exit}
        reading {sub(/^[[:space:]]*/, ""); print}
    ')"
    expected_arguments="$(printf '%s\n' "$binary" automation daemon run --registry "$registry" --root "$root" --periodic-seconds "$periodic_seconds")"
    [[ "$actual_path" == "$plist" && "$actual_program" == "$binary" && "$actual_arguments" == "$expected_arguments" ]]
}
assert_loaded_job_matches() {
    local snapshot
    if snapshot="$(/bin/launchctl print "$domain" 2>/dev/null)"; then
        loaded_job_matches "$snapshot" || die "loaded label has a different plist, executable or arguments; preserve it"
    fi
}
print_status() {
    local installed=false loaded=false running=false matches=false disabled=unknown state="" launch_status="" autostart=false
    [[ -f "$plist" ]] && installed=true
    if launch_status="$(/bin/launchctl print "$domain" 2>&1)"; then
        loaded=true
        if loaded_job_matches "$launch_status"; then matches=true; fi
        state="$(printf '%s\n' "$launch_status" | awk '/^[[:space:]]*state = / {print $3; exit}')"
        [[ "$state" == running && "$matches" == true ]] && running=true
    fi
    disabled="$(/bin/launchctl print-disabled "gui/$(id -u)" | awk -v key="\"$label\"" '$1 == key {print $3; exit}')"
    [[ -n "$disabled" ]] || disabled=false
    local login_directory=false
    if [[ -d "$agent_dir" && -d "$HOME/Library/LaunchAgents" ]] && \
       [[ "$(cd "$agent_dir" && pwd -P)" == "$(cd "$HOME/Library/LaunchAgents" && pwd -P)" ]]; then
        login_directory=true
    fi
    local configured=false
    [[ "$installed" == false || "$login_directory" == false ]] || configured=true
    [[ "$configured" == false || "$disabled" != false ]] || autostart=true
    printf 'launch_agent_label=%s\nlaunch_agent_plist=%s\nlaunch_agent_installed=%s\nlaunch_agent_loaded=%s\nlaunch_agent_target_matches=%s\nlaunch_agent_running=%s\nlaunch_agent_disabled=%s\nautostart_configured=%s\nautostart_enabled=%s\n' \
        "$label" "$plist" "$installed" "$loaded" "$matches" "$running" "$disabled" "$configured" "$autostart"
    [[ "$loaded" == false ]] || printf '%s\n' "$launch_status"
}
case "$action" in
    install)
        stable_gate
        private_directory "$root"
        private_directory "$root/runtime"
        private_directory "$log_dir"
        [[ -d "$agent_dir" && -O "$agent_dir" ]] || (umask 077; mkdir -p "$agent_dir")
        [[ -O "$agent_dir" && ! -L "$agent_dir" ]] || die "unsafe LaunchAgents directory"
        agent_mode="$(stat -f '%Lp' "$agent_dir")"
        (( (8#$agent_mode & 022) == 0 )) || die "LaunchAgents directory is writable by another user"
        if [[ ! -e "$plist" ]]; then
            candidate="$(mktemp "$agent_dir/.workvcs-plist.XXXXXX")"
            trap '[[ -z "${candidate:-}" ]] || rm -f "$candidate"' EXIT
            render > "$candidate"
            /usr/bin/plutil -lint "$candidate" >/dev/null || die "invalid generated plist"
            chmod 600 "$candidate"
            # A concurrent creator is never overwritten, even after preflight.
            ln "$candidate" "$plist" || die "plist appeared during installation"
            rm -f "$candidate"
            candidate=""
        fi
        printf 'launch_agent_install_status=verified\nlaunch_agent_start_requested=false\n'
        ;;
    start)
        stable_gate
        [[ -f "$plist" ]] || die "install the exact plist before starting"
        check_directory "$agent_dir" false
        for directory in "$root" "$root/runtime" "$log_dir"; do
            check_directory "$directory" true
            [[ -w "$directory" ]] || die "runtime directory is not writable: $directory"
        done
        assert_loaded_job_matches
        /bin/launchctl enable "$domain"
        if ! /bin/launchctl print "$domain" >/dev/null 2>&1; then
            /bin/launchctl bootstrap "gui/$(id -u)" "$plist"
        fi
        /bin/launchctl kickstart "$domain"
        started=false
        for ((attempt=0; attempt<50; attempt++)); do
            snapshot="$(/bin/launchctl print "$domain" 2>/dev/null)" || snapshot=""
            if loaded_job_matches "$snapshot" && \
               [[ "$(printf '%s\n' "$snapshot" | awk '/^[[:space:]]*state = / {print $3; exit}')" == running ]]; then
                started=true
                break
            fi
            sleep 0.1
        done
        print_status
        [[ "$started" == true ]] || die "launchd did not show the exact job running; inspect the reported state and stderr log"
        ;;
    stop|disable)
        [[ -f "$plist" ]] || die "managed plist is unavailable"
        assert_loaded_job_matches
        [[ "$action" != disable ]] || /bin/launchctl disable "$domain"
        if /bin/launchctl print "$domain" >/dev/null 2>&1; then /bin/launchctl bootout "$domain"; fi
        print_status
        ;;
    status) print_status ;;
esac
