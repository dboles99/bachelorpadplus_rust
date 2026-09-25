#!/usr/bin/env bash
# End-to-end smoke test of the real bachelorpad binary on a virtual X display.
#
# The headless window tests (crates/bp-ui/src/window_tests.rs) drive the Slint
# component through Slint's testing backend. They never load winit, never see
# an X keymap or focus, never write through the real save path to a real disk,
# and never receive a window manager's close request. This does all four:
# it starts Xvfb, launches the built binary with a throwaway home, types with
# XTEST, presses Ctrl+S, reads the file back, and answers the in-window
# "Unsaved changes" sheet after a WM_DELETE_WINDOW. See docs/governance/LOCAL_CI.md.
#
# Usage:
#   scripts/smoke-linux.sh [--binary PATH] [--view default|editor-view|all] [--keep]
#
# The binary defaults to $BACHELORPAD_BIN, else <target-dir>/debug/bachelorpad,
# where <target-dir> is $CARGO_TARGET_DIR or the repository's target/. It is
# never built here: building is the gate's job, and a smoke test that quietly
# rebuilds would test whatever it just built rather than what was handed to it.
#
# Needs: Xvfb, python3 with python-xlib, and libxkbcommon-x11 (winit dlopens
# it for X11 keyboard handling; without it the window opens and takes no keys).
#
# Exit status: 0 every view passed; 1 a check failed; 2 a prerequisite or the
# harness itself failed.

set -euo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(dirname -- "$here")

binary=${BACHELORPAD_BIN:-}
views=(default editor-view)
keep=()

while (($#)); do
    case $1 in
        --binary) binary=${2:?--binary needs a path}; shift 2 ;;
        --view)
            case ${2:-} in
                all) views=(default editor-view) ;;
                default | editor-view) views=("$2") ;;
                *) echo "FAIL  [args] --view takes default, editor-view or all" >&2; exit 2 ;;
            esac
            shift 2 ;;
        --keep) keep=(--keep); shift ;;
        -h | --help) sed -n '2,/^$/s/^# \{0,1\}//p' "$0"; exit 0 ;;
        *) echo "FAIL  [args] unknown argument: $1" >&2; exit 2 ;;
    esac
done

if [[ -z $binary ]]; then
    target=${CARGO_TARGET_DIR:-target}
    # A relative CARGO_TARGET_DIR is relative to where cargo ran, which for
    # this repository's scripts is the repository root.
    [[ $target == /* ]] || target=$repo/$target
    binary=$target/debug/bachelorpad
fi

started=$SECONDS
failed=0

pass() { printf 'PASS  [%s] %s\n' "$1" "$2"; }
fail() { printf 'FAIL  [%s] %s\n' "$1" "$2"; }

# --- prerequisites ----------------------------------------------------------
# Each is checked up front with its own message, because the symptom of a
# missing one further in is a window that never appears or takes no keys --
# which reads like a product defect.
prereq_ok=1
if [[ -x $binary ]]; then pass prereq "binary $binary"; else fail prereq "no executable at $binary (build it, or pass --binary)"; prereq_ok=0; fi
if command -v Xvfb >/dev/null; then pass prereq "Xvfb"; else fail prereq "Xvfb not on PATH (apt-get install xvfb)"; prereq_ok=0; fi
if python3 -c 'import Xlib.ext.xtest' 2>/dev/null; then
    pass prereq "python3 with python-xlib"
else
    fail prereq "python3 cannot import Xlib (apt-get install python3-xlib, or pip install python-xlib)"
    prereq_ok=0
fi
# Captured first rather than piped into `grep -q`: grep stops at the first
# match, ldconfig takes SIGPIPE, and pipefail turns a found library into a
# failed check -- intermittently, depending on where in the cache it sits.
libs=$(ldconfig -p 2>/dev/null || true)
if grep -q 'libxkbcommon-x11\.so\.0' <<<"$libs"; then
    pass prereq "libxkbcommon-x11"
else
    fail prereq "libxkbcommon-x11.so.0 not found (apt-get install libxkbcommon-x11-0)"
    prereq_ok=0
fi
((prereq_ok)) || exit 2

scratch=$(mktemp -d "${TMPDIR:-/tmp}/bp-smoke.XXXXXX")
xvfb_pid=

cleanup() {
    local status=$?
    if [[ -n $xvfb_pid ]] && kill -0 "$xvfb_pid" 2>/dev/null; then
        kill "$xvfb_pid" 2>/dev/null || true
        wait "$xvfb_pid" 2>/dev/null || true
    fi
    rm -rf -- "$scratch"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# --- the binary loads at all ------------------------------------------------
# The cheapest possible failure first: if the linked executable cannot reach
# main, everything after this would report a missing window instead.
if out=$(HOME=$scratch/selfcheck-home XDG_CONFIG_HOME=$scratch/selfcheck-home/config \
    XDG_STATE_HOME=$scratch/selfcheck-home/state XDG_DATA_HOME=$scratch/selfcheck-home/data \
    XDG_CACHE_HOME=$scratch/selfcheck-home/cache timeout 30 "$binary" --self-check 2>&1) &&
    grep -q BACHELORPAD_SELF_CHECK_OK <<<"$out"; then
    pass self-check "binary reached main and printed its marker"
else
    fail self-check "--self-check did not print BACHELORPAD_SELF_CHECK_OK"
    printf '        %s\n' "$out" | tail -n 15
    exit 1
fi

# --- a display of our own ---------------------------------------------------
# -displayfd lets Xvfb pick a free display number and report it, so two runs
# (or a developer's own :99) never collide, and there is no guess-and-retry
# loop to race. No -auth: local connections only (-nolisten tcp), and the
# server lives only as long as this script.
exec {dfd}>"$scratch/display"
Xvfb -displayfd "$dfd" -screen 0 1280x800x24 -nolisten tcp -noreset \
    >"$scratch/xvfb.log" 2>&1 &
xvfb_pid=$!
exec {dfd}>&-

for _ in $(seq 1 100); do
    [[ -s $scratch/display ]] && break
    kill -0 "$xvfb_pid" 2>/dev/null || break
    sleep 0.1
done
display_number=$(tr -dc '0-9' <"$scratch/display" || true)
if [[ -z $display_number ]]; then
    fail xvfb "Xvfb did not report a display within 10 s"
    sed 's/^/        /' "$scratch/xvfb.log" | tail -n 15
    exit 2
fi
export DISPLAY=:$display_number
unset WAYLAND_DISPLAY
pass xvfb "virtual display $DISPLAY"

# --- each view end to end ---------------------------------------------------
for view in "${views[@]}"; do
    status=0
    # A bound on the whole view as well as on each wait inside it, so a
    # harness bug cannot hang the run.
    timeout 60 python3 "$here/smoke_linux.py" "$binary" --view "$view" "${keep[@]}" || status=$?
    case $status in
        0) ;;
        124) fail "$view" "timed out after 60 s"; failed=1 ;;
        2) failed=2 ;;
        *) ((failed)) || failed=1 ;;
    esac
done

elapsed=$((SECONDS - started))
if ((failed)); then
    printf 'FAIL  smoke-linux: %s in %d s\n' "at least one check failed" "$elapsed"
    exit "$failed"
fi
printf 'PASS  smoke-linux: %s in %d s\n' "${views[*]}" "$elapsed"
