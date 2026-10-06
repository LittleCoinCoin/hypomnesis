# PR B's macos-latest CI: which `cli_ps` acceptance branch fired

run: 37210951264

PR #7 (mi-for-the-rust-of-us/hypomnesis), head fdf2f6cd5605947cb9ce7002197184c6eacb54a3, all 8 jobs success. The lines below are every line of `gh run view 37210951264 --repo mi-for-the-rust-of-us/hypomnesis --log` that starts with `macos-latest` and carries a `cli_ps` branch label, verbatim (job, step, timestamp, line; tab-separated):

```
macos-latest / 1.88	UNKNOWN STEP	2026-10-04T14:54:45.7310400Z cli_ps: with --exit-status branch=expected
macos-latest / 1.88	UNKNOWN STEP	2026-10-04T14:54:45.7667170Z cli_ps: without --exit-status branch=expected
macos-latest / stable	UNKNOWN STEP	2026-10-04T14:54:20.3860350Z cli_ps: with --exit-status branch=expected
macos-latest / stable	UNKNOWN STEP	2026-10-04T14:54:20.4006760Z cli_ps: without --exit-status branch=expected
```

On both macos-latest runners (`1.88` and `stable`), `branch=expected` fired for both `with --exit-status` and `without --exit-status`: the exit code matched the old rule, so `hmn ps` could query the VM's device and no runner took the `skipped-device` (exit 2 with the skip line) branch; none of the four is the rejected branch.
