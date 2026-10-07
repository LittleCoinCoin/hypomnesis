# hmn PR C (c026a93) inside Claude Code's Bash sandbox (macOS, Apple M3 Pro)

enabled-by: /sandbox; user-confirmed: yes

```
claude-code: 2.1.273 (Claude Code)
hmn-commit: c026a93dac3b8a6bc68a63219d29a38b756d9f54
sandbox-exec: sandbox_apply: Operation not permitted
nested sandbox-exec rc=71
envsandbox: SANDBOX_RUNTIME=1
responsible pid 31126
pid 393: ledger rc=0 errno=0  proc_pidpath ret=18 errno=0
pid 1: ledger rc=0 errno=0  proc_pidpath ret=13 errno=0
pid 2561: ledger rc=-1 errno=ESRCH  proc_pidpath ret=0 errno=ESRCH
pid 32272: ledger rc=0 errno=0  proc_pidpath ret=39 errno=0
sandboxed: 1
hmn 0.2.13
PID    NAME                                          VRAM     SHARED  DEVICE        SPILL
<28 rows omitted: raw process listing not committed>
hmn: 28 GPU processes found (1.4 GiB committed total).
ps rc=0
PID    NAME                                          VRAM     SHARED  DEVICE        SPILL
<28 rows omitted: raw process listing not committed>
hmn: 28 GPU processes found matching device=0 (1.4 GiB committed total).
ps --device 0 rc=0
hmn watch: device 0 [Apple M3 Pro], interval 1.0s, watching 1 PID(s)
TIME      PID      NAME          COMMITTED  ΔCOMMIT    SHARED     ΔSHARED    SPILL
+0.0s     1        ?             0 MiB      +0 B       0 MiB      +0 B       n/a  
hmn watch: spill not measurable on this platform; per-PID VRAM below
hmn watch: per-PID  PID  NAME  BASELINE COMMIT  PEAK COMMIT  BASELINE SHARED  PEAK SHARED  PAGED
                    1    ?     0 MiB            0 MiB        0 MiB            0 MiB        n/a  
watch rc=0
sysctl kern.proc.all size probe 0 None 590328
fill 0 None kinfo_proc count 906
```

outcome: full

Inside Claude Code's Bash sandbox, `hmn ps` listed all 28 GPU processes with their VRAM, with or without `--device 0`, and reported nothing unreadable, and `hmn watch 1` ran for its full duration and exited 0.
