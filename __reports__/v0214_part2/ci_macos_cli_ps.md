# PR C's macos-latest CI: which `cli_ps` acceptance branch fired

run: 37650659228

PR #8 (mi-for-the-rust-of-us/hypomnesis), head c026a93dac3b8a6bc68a63219d29a38b756d9f54, all 8 jobs success. The lines below are every line of `gh run view 37650659228 --repo mi-for-the-rust-of-us/hypomnesis --log` that starts with `macos-latest` and carries a `cli_ps` branch label, verbatim (job, step, timestamp, line; tab-separated):

```
macos-latest / 1.88	UNKNOWN STEP	2026-10-07T16:16:51.9546900Z cli_ps: with --exit-status branch=expected
macos-latest / 1.88	UNKNOWN STEP	2026-10-07T16:16:51.9840120Z cli_ps: without --exit-status branch=expected
macos-latest / stable	UNKNOWN STEP	2026-10-07T16:16:43.2101930Z cli_ps: with --exit-status branch=expected
macos-latest / stable	UNKNOWN STEP	2026-10-07T16:16:43.2415970Z cli_ps: without --exit-status branch=expected
```

On both macos-latest runners (`1.88` and `stable`), `branch=expected` fired for both `with --exit-status` and `without --exit-status`: the exit code matched the old rule, so `hmn ps` could query the VM's device and no runner took the `skipped-device` (exit 2 with the denial line) or `skipped-device-nogpu` (exit 2 with a skip line carrying the `NoGpuSource` text) branch; none of the four is the rejected branch.
