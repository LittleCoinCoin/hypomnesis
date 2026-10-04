# SPDX-License-Identifier: MIT OR Apache-2.0
"""capture.py <hmn-binary> <outdir>

Run six `hmn` probes under the policies none, P and C (through sandbox.sh)
and write <outdir>/<policy>/<probe>.{stdout,stderr,exit}, normalised so that
no process name, size, count or time is committed. The raw output goes to
$HMN_CAPTURE_RAW (default: a fresh temporary directory), which must lie
outside the repository. Use a release binary built with
`cargo build --release --locked` (default features): an --all-features build
prints `[nvidia-smi debug]` lines on stderr.

Normalisation (README.md beside this file has the full rules):
  - `[nvidia-smi debug]` lines are dropped (.stdout and .stderr);
  - table probes' .stdout becomes a header `PID<TAB>NAME<TAB>DEVICE<TAB>SPILL`
    and one row per process, sliced by the header's column starts, NAME
    replaced by the first 8 hex digits of its SHA-256 (`?` stays `?`), VRAM
    and SHARED dropped;
  - ps_json's .stdout becomes `keys=<sorted key names>` (empty if hmn printed
    nothing);
  - everywhere else: sizes -> `<size>`, `N GPU process(es) found` ->
    `<N> GPU process(es) found`, `+N.Ns` -> `+<t>s`, runs of spaces collapse
    to one and trailing spaces are removed.

Exit codes: 0 on success, 64 on a usage error or a raw directory inside the
repository. Python 3, standard library only.
"""

import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
SANDBOX = os.path.join(HERE, "sandbox.sh")

POLICIES = ["none", "P", "C"]
PROBES = {
    "ps": ["ps"],
    "ps_json": ["ps", "--json"],
    "ps_device_0": ["ps", "--device", "0"],
    "ps_pid_max_exit_status": ["ps", "--pid", "4294967295", "--exit-status"],
    "ps_filter_windowserver": ["ps", "--filter", "WindowServer"],
    "watch_0": ["watch", "0", "--duration", "2s", "--interval", "1s"],
}
TABLE_PROBES = {"ps", "ps_device_0", "ps_pid_max_exit_status", "ps_filter_windowserver"}

DEBUG = "[nvidia-smi debug]"
SIZE = re.compile(r"\d+(\.\d+)? (B|KiB|MiB|GiB)\b")
COUNT = re.compile(r"\b\d+ GPU process(es)? found")
TIME = re.compile(r"\+\d+(\.\d+)?s\b")
SPACES = re.compile(r" {2,}")


def usage(msg=None):
    if msg:
        print("capture.py: " + msg, file=sys.stderr)
    print("usage: capture.py <hmn-binary> <outdir>", file=sys.stderr)
    sys.exit(64)


def repo_roots():
    """The repository holding this script, lexical and resolved; empty if none."""
    try:
        top = subprocess.run(["git", "-C", HERE, "rev-parse", "--show-toplevel"],
                             capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return []
    return [top, os.path.realpath(top)] if top else []


def inside(path, roots):
    return any(path == r or path.startswith(r + os.sep) for r in roots)


def keep(lines):
    return [l for l in lines if not l.startswith(DEBUG)]


def mask(line):
    line = SIZE.sub("<size>", line)
    line = COUNT.sub("<N> GPU process(es) found", line)
    line = TIME.sub("+<t>s", line)
    return SPACES.sub(" ", line).rstrip()


def name_hash(name):
    if name in ("?", ""):
        return name
    return hashlib.sha256(name.encode("utf-8")).hexdigest()[:8]


def table(lines):
    if not lines:
        return []
    header = lines[0]
    cols = [(m.start(), m.group()) for m in re.finditer(r"\S+", header)]
    names = [c[1] for c in cols]
    want = ["PID", "NAME", "DEVICE", "SPILL"]
    if not all(w in names for w in want):
        # Not a table (e.g. an error text on stdout): mask it like any text.
        return [mask(l) for l in lines]
    starts = [c[0] for c in cols] + [None]
    res = ["\t".join(want)]
    for line in lines[1:]:
        cells = {n: line[starts[i]:starts[i + 1]].strip() for i, n in enumerate(names)}
        res.append("\t".join([cells["PID"], name_hash(cells["NAME"]),
                              cells["DEVICE"], cells["SPILL"]]))
    return res


def ps_json(text):
    if not text.strip():
        return []
    try:
        rows = json.loads(text)
    except ValueError:
        return [mask(l) for l in keep(text.splitlines())]
    return ["keys=" + ",".join(sorted({k for r in rows for k in r}))]


def write_lines(path, lines):
    with open(path, "w", encoding="utf-8") as f:
        f.write("".join(l + "\n" for l in lines))


def main(argv):
    if len(argv) != 2:
        usage()
    binary, out = argv
    if not (os.path.isfile(binary) and os.access(binary, os.X_OK)):
        usage("'%s' is not an executable" % binary)
    binary = os.path.abspath(binary)

    raw = os.environ.get("HMN_CAPTURE_RAW") or tempfile.mkdtemp()
    raw = os.path.abspath(raw)
    roots = repo_roots()
    if inside(raw, roots):
        usage("HMN_CAPTURE_RAW (%s) is inside the repository; refusing" % raw)
    os.makedirs(raw, exist_ok=True)
    raw = os.path.realpath(raw)
    if inside(raw, roots):
        usage("HMN_CAPTURE_RAW (%s) is inside the repository; refusing" % raw)

    for policy in POLICIES:
        os.makedirs(os.path.join(raw, policy), exist_ok=True)
        os.makedirs(os.path.join(out, policy), exist_ok=True)
        for probe, args in PROBES.items():
            base = os.path.join(raw, policy, probe)
            with open(base + ".stdout", "wb") as so, open(base + ".stderr", "wb") as se:
                rc = subprocess.run(["bash", SANDBOX, policy, "--", binary] + args,
                                    stdout=so, stderr=se, check=False).returncode
            if rc < 0:
                rc = 128 - rc  # killed by a signal: the shell's 128+N convention
            with open(base + ".exit", "w") as f:
                f.write("%d\n" % rc)

    for policy in POLICIES:
        for probe in PROBES:
            base = os.path.join(raw, policy, probe)
            dest = os.path.join(out, policy, probe)
            with open(base + ".stdout", encoding="utf-8", errors="replace") as f:
                stdout = f.read()
            with open(base + ".stderr", encoding="utf-8", errors="replace") as f:
                stderr = keep(f.read().splitlines())
            if probe in TABLE_PROBES:
                so = table(keep(stdout.splitlines()))
            elif probe == "ps_json":
                so = ps_json(stdout)
            else:
                so = [mask(l) for l in keep(stdout.splitlines())]
            write_lines(dest + ".stdout", so)
            write_lines(dest + ".stderr", [mask(l) for l in stderr])
            with open(base + ".exit") as f:
                write_lines(dest + ".exit", [f.read().strip()])

    print("capture.py: raw output in " + raw, file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
