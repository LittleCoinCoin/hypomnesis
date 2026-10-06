# SPDX-License-Identifier: MIT OR Apache-2.0
"""compare.py <base> <new> [--spill-map]

The byte-identity instrument of the v0.2.14 campaign: compare two capture
directories written by capture.py, for the policies `none` and `C` (the two
whose output must not change), file by file:
  - .exit equal;
  - .stderr equal, after removing from <base> the allowlisted lines (each
    removed line is counted);
  - table .stdout (ps, ps_device_0, ps_pid_max_exit_status,
    ps_filter_windowserver): header equal, then per PID over the PIDs present
    in both captures, NAME hash, DEVICE and SPILL equal;
  - every other .stdout equal line by line.
With --spill-map, a SPILL cell (table rows) or the last cell of a watch_0 line
(its SPILL and its per-PID PAGED cell) that is `?` in <base> and `n/a` in
<new> is not a difference, and is counted. Policy P is not compared: it
changes by design in PR B and in PR C.

Prints one line per policy, `<policy>: identical after normalisation (rows
compared: N, spill cells mapped: M, allowlisted lines: K)` or `<policy>: DIFF`
followed by the differences. Exit 0 if both policies are identical, 1 on a
difference, 2 on a usage error. Python 3, standard library only.
"""

import os
import sys

POLICIES = ["none", "C"]
PROBES = ["ps", "ps_json", "ps_device_0", "ps_pid_max_exit_status",
          "ps_filter_windowserver", "watch_0"]
TABLE_PROBES = {"ps", "ps_device_0", "ps_pid_max_exit_status", "ps_filter_windowserver"}
# Lines removed from <base>'s .stderr before comparing. Each entry is a
# substring; each removed line is counted as allowlisted.
#   - `names no running process`: the `hmn watch 0` warning that PR B's
#     process_exists fix removes (kernel_task has no executable path).
STDERR_ALLOWLIST = ["names no running process"]


def usage(msg=None):
    if msg:
        print("compare.py: " + msg, file=sys.stderr)
    print("usage: compare.py <base> <new> [--spill-map]", file=sys.stderr)
    sys.exit(2)


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read().splitlines()


def parse_table(lines):
    """PID -> [pid, name hash, device, spill]; None if a row is not 4 cells."""
    rows = {}
    for line in lines[1:]:
        cells = line.split("\t")
        if len(cells) != 4:
            return None
        rows[cells[0]] = cells
    return rows


def compare_policy(base, new, policy, spill_map):
    diffs = []
    counts = {"rows": 0, "mapped": 0, "allowed": 0}

    def spill_ok(b, n):
        return spill_map and b == "?" and n == "n/a"

    for probe in PROBES:
        for ext in (".exit", ".stderr", ".stdout"):
            bp = os.path.join(base, policy, probe + ext)
            np_ = os.path.join(new, policy, probe + ext)
            missing = [p for p in (bp, np_) if not os.path.isfile(p)]
            if missing:
                diffs.append("%s%s: missing %s" % (probe, ext, ", ".join(missing)))
                continue
            b, n = read(bp), read(np_)
            if ext == ".stderr":
                kept = [l for l in b if not any(a in l for a in STDERR_ALLOWLIST)]
                counts["allowed"] += len(b) - len(kept)
                b = kept
            if ext == ".stdout" and probe in TABLE_PROBES:
                if b[:1] != n[:1]:
                    diffs.append("%s.stdout: header %r != %r" % (probe, b[:1], n[:1]))
                    continue
                bd, nd = parse_table(b), parse_table(n)
                if bd is None or nd is None:
                    if b != n:
                        diffs.append("%s.stdout: not a table and not equal" % probe)
                    continue
                for pid in sorted(set(bd) & set(nd), key=int):
                    counts["rows"] += 1
                    bc, nc = bd[pid], nd[pid]
                    if bc[1:3] != nc[1:3]:
                        diffs.append("%s.stdout: pid %s: %r != %r" % (probe, pid, bc[1:3], nc[1:3]))
                    elif bc[3] != nc[3]:
                        if spill_ok(bc[3], nc[3]):
                            counts["mapped"] += 1
                        else:
                            diffs.append("%s.stdout: pid %s: SPILL %r != %r"
                                         % (probe, pid, bc[3], nc[3]))
                continue
            if len(b) != len(n):
                diffs.append("%s%s: %d lines != %d lines" % (probe, ext, len(b), len(n)))
                continue
            for i, (bl, nl) in enumerate(zip(b, n), 1):
                if bl == nl:
                    continue
                if ext == ".stdout" and probe == "watch_0":
                    bt, nt = bl.split(" "), nl.split(" ")
                    if bt[:-1] == nt[:-1] and spill_ok(bt[-1], nt[-1]):
                        counts["mapped"] += 1
                        continue
                diffs.append("%s%s:%d: %r != %r" % (probe, ext, i, bl, nl))
    return diffs, counts


def main(argv):
    spill_map = False
    args = []
    for a in argv:
        if a == "--spill-map":
            spill_map = True
        elif a.startswith("-"):
            usage("unknown option '%s'" % a)
        else:
            args.append(a)
    if len(args) != 2:
        usage()
    base, new = args
    for d in (base, new):
        if not os.path.isdir(d):
            usage("no such directory: " + d)

    worst = 0
    for policy in POLICIES:
        diffs, c = compare_policy(base, new, policy, spill_map)
        if diffs:
            worst = 1
            print("%s: DIFF" % policy)
            for d in diffs:
                print("  " + d)
        else:
            print("%s: identical after normalisation (rows compared: %d, "
                  "spill cells mapped: %d, allowlisted lines: %d)"
                  % (policy, c["rows"], c["mapped"], c["allowed"]))
    return worst


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
