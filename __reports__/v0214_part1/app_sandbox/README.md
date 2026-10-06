# `hmn` inside a real App Sandbox

Fixtures (15) and (16) of [`00-findings_v0.md`](../00-findings_v0.md): `hmn ps` and
`hmn ps --device 0` run by an `hmn` that the App Sandbox confines at launch, for c1810a5
and for PR B. Measured 2026-10-04 on the M3 Pro (macOS 26.6.2).

## Build

The recipe of [`field_check_v0213/evidence/probes/appsandbox`](../../field_check_v0213/evidence/probes/appsandbox/README.md),
with its `Info.plist` (bundle id `org.example.hmnsb`) and `entitlements.plist`
(`com.apple.security.app-sandbox`), applied to a default-feature release build of each
tree. The build goes to a separate target directory inside that tree's own `target/`, so
the plain release binary that `capture.py` uses is not overwritten:

```bash
PL=__reports__/field_check_v0213/evidence/probes/appsandbox   # absolute path in practice
RUSTFLAGS="-C link-arg=-Wl,-sectcreate,__TEXT,__info_plist,$PL/Info.plist" \
    cargo build --release --locked --target-dir target/appsandbox
codesign -s - -f --entitlements $PL/entitlements.plist target/appsandbox/release/hmn
```

Run from a copy outside `~/Documents`. Run in place under `~/Documents/...`, the
App-Sandboxed binary dies with `SIGTRAP` (exit 133) before `main`, even for `--version`,
for both trees. The likely cause is the App Sandbox refusing the executable's location
under the TCC-protected `~/Documents`; it was not investigated further, since copied into
a scratch directory under `/private/tmp` both builds run.

Proof that both copies were App-Sandboxed:

- `codesign -d --entitlements - --xml` shows `com.apple.security.app-sandbox` `true`, and
  `otool -s __TEXT __info_plist -V` shows the embedded `org.example.hmnsb` plist, for both.
- The container `~/Library/Containers/org.example.hmnsb` exists.
- The C probe of the same recipe, built and run from the same scratch directory, printed
  `proc_listpids size probe: 0 errno=Operation not permitted`.
- The same source, built without the plist and the entitlement, lists 19 processes
  unsandboxed (harness README, profile `none`).

## Results

The full output is in [`c1810a5.txt`](c1810a5.txt) and [`pr_b.txt`](pr_b.txt): stderr
verbatim, and stdout reduced to its line and byte counts so that no process list is
committed (no command listed a process: c1810a5's `ps` stdout is the 39-byte table header
alone).

| Command | c1810a5 | PR B |
|:--------|:--------|:-----|
| `hmn ps` | exit 0, `hmn: 0 GPU processes found.` | exit 2, `hmn: ps failed to query device 0: no GPU measurement source available (Metal, NVML, and nvidia-smi all failed or are disabled) (skipped)`, then `hmn: ps: no device could be queried, so nothing could be listed`, no stdout |
| `hmn ps --device 0` | exit 2, `NoGpuSource` naming NVML, DXGI, PDH and `nvidia-smi` | exit 2, `hmn: ps failed to query device 0: no GPU measurement source available (Metal, NVML, and nvidia-smi all failed or are disabled)` |
| `hmn ps --exit-status` | exit 1 | exit 2 |
| `hmn ps --json` | exit 0, `[]` (3 bytes) | exit 2, no stdout |
| `hmn ps --device 1` | exit 2, the `NoGpuSource` text | exit 2, `device index 1 out of range (have 1 devices)` |
| `hmn --version` | exit 0 | exit 0 |
