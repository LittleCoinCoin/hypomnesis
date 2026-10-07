# SPDX-License-Identifier: MIT OR Apache-2.0
"""count_denied.py

An independent probe of what the calling sandbox lets a process read: it
enumerates every process with `sysctl` MIB {CTL_KERN, KERN_PROC, KERN_PROC_ALL}
(`kinfo_proc` records of 648 bytes, `p_pid` at offset 40), reads each PID's
ledger (`ledger` command LEDGER_ENTRY_INFO_V2 = 4) and prints

    denied=<n> read=<m> gone=<k>

`EPERM` counts as denied, `ESRCH` as gone, a successful read as read. The
probe's own PID is never counted. An `errno` of any other kind is printed as a
trailing ` other=<j>` and never added to one of the three counts. It shares no
code with `hmn`, so a count of `hmn`'s own can be compared with it.

Exit codes: 0 on success, 1 when the process table cannot be listed (the
`sysctl` fails: profile Q, say), when its length is not a whole number of
648-byte records, or when it does not hold the probe's own PID (so an empty or
mis-parsed listing never prints a line). Python 3, standard library only.
"""

import ctypes
import errno as errno_codes
import os
import struct
import sys

CTL_KERN = 1
KERN_PROC = 14
KERN_PROC_ALL = 0
KINFO_PROC_SIZE = 648
P_PID_OFFSET = 40
LEDGER_ENTRY_INFO_V2 = 4
LEDGER_ENTRY_INFO_V2_SIZE = 88
LEDGER_ENTRIES = 128

libc = ctypes.CDLL(None, use_errno=True)
libc.sysctl.argtypes = [
    ctypes.POINTER(ctypes.c_int), ctypes.c_uint,
    ctypes.c_void_p, ctypes.POINTER(ctypes.c_size_t),
    ctypes.c_void_p, ctypes.c_size_t,
]
libc.sysctl.restype = ctypes.c_int
libc.ledger.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
libc.ledger.restype = ctypes.c_int


def list_pids():
    """Every PID `KERN_PROC_ALL` lists, or None when the call fails or its
    answer is not whole records."""
    mib = (ctypes.c_int * 3)(CTL_KERN, KERN_PROC, KERN_PROC_ALL)
    for _ in range(4):
        size = ctypes.c_size_t(0)
        if libc.sysctl(mib, 3, None, ctypes.byref(size), None, 0) != 0:
            return None
        records = size.value // KINFO_PROC_SIZE + 16
        buf = ctypes.create_string_buffer(records * KINFO_PROC_SIZE)
        size = ctypes.c_size_t(len(buf))
        if libc.sysctl(mib, 3, buf, ctypes.byref(size), None, 0) != 0:
            if ctypes.get_errno() == errno_codes.ENOMEM:
                continue
            return None
        raw = buf.raw[: size.value]
        if len(raw) % KINFO_PROC_SIZE != 0:
            return None
        return [
            struct.unpack_from("=i", raw, i * KINFO_PROC_SIZE + P_PID_OFFSET)[0]
            for i in range(len(raw) // KINFO_PROC_SIZE)
        ]
    return None


def ledger_errno(pid):
    """0 when `pid`'s ledger can be read, else the `errno` of the refusal."""
    entries = ctypes.create_string_buffer(LEDGER_ENTRIES * LEDGER_ENTRY_INFO_V2_SIZE)
    count = ctypes.c_int(LEDGER_ENTRIES)
    if libc.ledger(LEDGER_ENTRY_INFO_V2, ctypes.c_void_p(pid), entries, ctypes.byref(count)) == 0:
        return 0
    return ctypes.get_errno()


def main():
    pids = list_pids()
    if pids is None:
        print("count_denied.py: sysctl KERN_PROC_ALL failed", file=sys.stderr)
        return 1
    me = os.getpid()
    if me not in pids:
        print("count_denied.py: the listing does not hold this process", file=sys.stderr)
        return 1
    denied = read = gone = other = 0
    for pid in pids:
        if pid <= 0 or pid == me:
            continue
        code = ledger_errno(pid)
        if code == 0:
            read += 1
        elif code == errno_codes.EPERM:
            denied += 1
        elif code == errno_codes.ESRCH:
            gone += 1
        else:
            other += 1
    line = "denied=%d read=%d gone=%d" % (denied, read, gone)
    if other:
        line += " other=%d" % other
    print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
