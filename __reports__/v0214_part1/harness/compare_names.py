# SPDX-License-Identifier: MIT OR Apache-2.0
"""compare_names.py <unsandboxed.json> <profile.json>

Compare the NAME column of two raw `hmn ps --json` outputs, each produced by
the command itself (not the normalised files capture.py commits, whose names
are hashes): one unsandboxed, one under a profile that withholds
`proc_pidpath`.

A PID is compared when it is in both captures and its name in the first is
not `null`. The second capture's name must then equal the first's, or equal
the first's name cut at 16 bytes (the kernel's `p_comm` limit) and then cut
back to its last whole UTF-8 character, which is what `hmn` returns for a
name the kernel cut inside a multibyte character. A `null` name in the second
capture is a mismatch, and so is any other prefix, the empty one included.
`hmn`'s own PID differs between two runs, so its row is never in both and is
not compared.

Prints `compared <N> mismatch <M>` on stdout and each mismatch (PID and names)
on stderr. Exit 0 when at least one PID was compared and none mismatched, 1
otherwise (an empty overlap, or two captures that name nothing, never
passes), 2 on a usage error or an unreadable input. Python 3, standard
library only.
"""

import json
import sys

P_COMM_MAX = 16


def load(path):
    """{pid: name} from a `hmn ps --json` file; exits 2 on a bad input."""
    try:
        with open(path, encoding="utf-8") as handle:
            rows = json.load(handle)
        return {row["pid"]: row.get("name") for row in rows}
    except (OSError, ValueError, KeyError, TypeError) as err:
        print("compare_names.py: cannot read %s: %s" % (path, err), file=sys.stderr)
        sys.exit(2)


def same_or_cut(full, other):
    """`other` is `full`, or `full` cut at 16 bytes and back to a character."""
    if other is None:
        return False
    if other == full:
        return True
    cut = full.encode("utf-8")[:P_COMM_MAX].decode("utf-8", "ignore")
    return other == cut


def main(argv):
    if len(argv) != 3:
        print("usage: compare_names.py <unsandboxed.json> <profile.json>", file=sys.stderr)
        return 2
    base = load(argv[1])
    other = load(argv[2])
    compared = mismatch = 0
    for pid in sorted(set(base) & set(other)):
        if base[pid] is None:
            continue
        compared += 1
        if not same_or_cut(base[pid], other[pid]):
            mismatch += 1
            print("pid %d: %r vs %r" % (pid, base[pid], other[pid]), file=sys.stderr)
    print("compared %d mismatch %d" % (compared, mismatch))
    return 0 if compared >= 1 and mismatch == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
