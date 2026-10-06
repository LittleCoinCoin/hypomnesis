#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# sandbox.sh [--print-profile] PROFILE [--] CMD [ARGS...]
#
# Run CMD under one of the v0.2.14 campaign's named Seatbelt profiles, or
# print the profile text. The exit code is CMD's; a usage error (an unknown
# option, an unknown profile, no CMD) exits 64. It never falls back to an
# unsandboxed run: only the profile `none` runs CMD outside a sandbox.
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
# PR C extends this file (a --job option and fixtures beside it); it does
# not fork it.

set -u

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
P_TEXT='(version 1)(allow default)(deny process-info*)(allow process-info* (target self))'

usage() {
    echo "usage: sandbox.sh [--print-profile] PROFILE [--] CMD [ARGS...]" >&2
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

if [ "${1:-}" = "--" ]; then
    shift
fi
if [ $# -lt 1 ]; then
    echo "sandbox.sh: no command given" >&2
    usage
    exit 64
fi

case "$profile" in
    none) exec "$@" ;;
    S) exec sandbox-exec -p "$text" bash -c '"$@"; rc=$?; exit $rc' _ "$@" ;;
    C) exec sandbox-exec -f "$HERE/codex.sb" "$@" ;;
    *) exec sandbox-exec -p "$text" "$@" ;;
esac
