#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# sandbox.sh [--print-profile] PROFILE [--job] [--] CMD [ARGS...]
#
# Run CMD under one of the v0.2.14 campaign's named Seatbelt profiles, or
# print the profile text. The exit code is CMD's; a usage error (an unknown
# option, an unknown profile, no CMD) exits 64. It never falls back to an
# unsandboxed run: only the profile `none` runs CMD outside a sandbox.
#
# --job (after PROFILE) starts gpujob, a resident 256 MiB Metal buffer, inside
# the same sandbox as CMD, waits 3 s, exports its PID as $JOB, runs CMD and
# kills the job. gpujob.swift is compiled on first use, outside the sandbox,
# into target/gpujob of the directory sandbox.sh is run from. Exit 70 when
# that compile fails or the job does not start.
#
# Profiles (see README.md beside this file):
#   none  CMD unsandboxed (exec)
#   P     the report's profile: process-info* denied except on the caller
#   Q     P, plus sysctl kern.proc* denied
#   S     P, plus process-info* allowed for the same sandbox, run under a
#         resident bash wrapper so CMD is not alone in its sandbox
#   S0    the text of S, run directly (CMD alone: behaves like P)
#   D     process-info-pidinfo denied except on the caller (proc_pidpath)
#   L     process-info-ledger denied for every process, the caller included
#   C     OpenAI Codex's base policy, pinned (codex.sb, CODEX_PIN)
#
# count_denied.py and compare_names.py beside this file are the fixtures that
# judge what hmn reports under these profiles.

set -u

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
P_TEXT='(version 1)(allow default)(deny process-info*)(allow process-info* (target self))'

usage() {
    echo "usage: sandbox.sh [--print-profile] PROFILE [--job] [--] CMD [ARGS...]" >&2
    echo "       PROFILE is one of: none P Q S S0 D L C" >&2
}

# profile_text NAME: print the profile's text on stdout; return 1 if NAME
# is not a known profile.
profile_text() {
    case "$1" in
        none) printf '\n' ;;
        P) printf '%s\n' "$P_TEXT" ;;
        Q) printf '%s\n' "${P_TEXT}(deny sysctl-read (sysctl-name-prefix \"kern.proc\"))" ;;
        S | S0) printf '%s\n' "${P_TEXT}(allow process-info* (target same-sandbox))" ;;
        D) printf '%s\n' '(version 1)(allow default)(deny process-info-pidinfo)(allow process-info-pidinfo (target self))' ;;
        L) printf '%s\n' '(version 1)(allow default)(deny process-info-ledger)' ;;
        C) cat "$HERE/codex.sb" ;;
        *) return 1 ;;
    esac
}

print_only=0
while [ $# -gt 0 ]; do
    case "$1" in
        --print-profile) print_only=1; shift ;;
        --) usage; exit 64 ;;
        -*) echo "sandbox.sh: unknown option '$1'" >&2; usage; exit 64 ;;
        *) break ;;
    esac
done

if [ $# -lt 1 ]; then
    usage
    exit 64
fi
profile="$1"
shift

if ! text="$(profile_text "$profile")"; then
    echo "sandbox.sh: unknown profile '$profile'" >&2
    exit 64
fi

if [ "$print_only" = 1 ]; then
    printf '%s\n' "$text"
    exit 0
fi

job=0
if [ "${1:-}" = "--job" ]; then
    job=1
    shift
fi

if [ "${1:-}" = "--" ]; then
    shift
fi
if [ $# -lt 1 ]; then
    echo "sandbox.sh: no command given" >&2
    usage
    exit 64
fi

if [ "$job" = 1 ]; then
    JOB_BIN="$PWD/target/gpujob"
    if [ ! -x "$JOB_BIN" ] || [ "$HERE/gpujob.swift" -nt "$JOB_BIN" ]; then
        mkdir -p "$PWD/target" && swiftc -O "$HERE/gpujob.swift" -o "$JOB_BIN" >&2 || {
            echo "sandbox.sh: cannot compile gpujob.swift" >&2
            exit 70
        }
    fi
    # One bash inside the profile's one sandbox-exec: the job and CMD share
    # the sandbox. The job's stdout goes to /dev/null so it cannot pollute
    # CMD's.
    JOB_SCRIPT='job_bin=$1; shift
"$job_bin" >/dev/null &
JOB=$!
export JOB
trap "kill \$JOB 2>/dev/null" EXIT
sleep 3
if ! kill -0 "$JOB" 2>/dev/null; then
    echo "sandbox.sh: gpujob did not start" >&2
    exit 70
fi
"$@"
exit $?'
    case "$profile" in
        none) exec bash -c "$JOB_SCRIPT" _ "$JOB_BIN" "$@" ;;
        C) exec sandbox-exec -f "$HERE/codex.sb" bash -c "$JOB_SCRIPT" _ "$JOB_BIN" "$@" ;;
        *) exec sandbox-exec -p "$text" bash -c "$JOB_SCRIPT" _ "$JOB_BIN" "$@" ;;
    esac
fi

case "$profile" in
    none) exec "$@" ;;
    S) exec sandbox-exec -p "$text" bash -c '"$@"; rc=$?; exit $rc' _ "$@" ;;
    C) exec sandbox-exec -f "$HERE/codex.sb" "$@" ;;
    *) exec sandbox-exec -p "$text" "$@" ;;
esac
