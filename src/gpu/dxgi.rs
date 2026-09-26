// SPDX-License-Identifier: MIT OR Apache-2.0

//! `DXGI` backend for per-process `VRAM` on Windows.
//!
//! Walks `IDXGIFactory1::EnumAdapters1`, filters NVIDIA adapters by
//! PCI vendor ID, casts to `IDXGIAdapter3`, and calls
//! `QueryVideoMemoryInfo(DXGI_MEMORY_SEGMENT_GROUP_LOCAL)` for the
//! per-process `CurrentUsage` figure. `WDDM`-aware: this is the only
//! reliable per-process `VRAM` source on Windows because the kernel
//! memory manager owns GPU allocations under `WDDM`, not the NVIDIA
//! driver — `NVML`'s `nvmlDeviceGetComputeRunningProcesses_v3` returns
//! `NVML_VALUE_NOT_AVAILABLE` for compute processes on `WDDM`.
//!
//! # Adapter filtering
//!
//! Two enumeration views coexist with opposite filters:
//!
//! - **NVIDIA-only** ([`query`], `adapter_name`, `adapter_luid`,
//!   `adapter_dedicated_video_memory`, [`device_count`]).
//!   `device_index: u32` is the index into the *filtered* list of NVIDIA
//!   adapters with non-zero dedicated `VRAM`. iGPUs (Intel `0x8086`, AMD
//!   integrated `0x1002`), the Microsoft Basic Render Driver (`0x1414`),
//!   and any non-NVIDIA discrete GPUs are skipped. Filter rule
//!   ([`is_nvidia_dgpu`]): `VendorId == 0x10DE` AND
//!   `DedicatedVideoMemory > 0`.
//!
//! - **Non-NVIDIA** ([`enumerate_non_nvidia`], used by
//!   `Snapshot::all` on Windows). Reverses the NVIDIA filter to surface
//!   AMD / Intel iGPUs and any non-NVIDIA discrete GPUs alongside the
//!   NVIDIA dGPU(s) `NVML` enumerates. Filter rule:
//!   `VendorId != 0x10DE` AND `VendorId != 0x1414` AND
//!   (`DedicatedVideoMemory > 0` OR `SharedSystemMemory > 0`).
//!
//! # One walker
//!
//! Both views are built on [`walk_adapters`], the module's only
//! `EnumAdapters1` loop (since v0.2.12; before that, six functions each
//! carried their own copy). It owns the adapter-skipping policy — an
//! adapter whose `IDXGIAdapter` cast or `GetDesc` fails is skipped, never
//! treated as the end of the walk — so every lookup agrees on which
//! adapters exist and how NVIDIA indices are numbered. It takes a visitor
//! closure rather than returning an iterator so that COM pointers never
//! leave its frame (`CONVENTIONS.md`, Pattern 3). New `DXGI` enumeration
//! goes through it, via [`nth_nvidia_adapter`] or [`for_each_adapter`]
//! where those fit.

use std::convert::Infallible;
use std::ops::ControlFlow;

use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ADAPTER_DESC, DXGI_MEMORY_SEGMENT_GROUP_LOCAL,
    DXGI_QUERY_VIDEO_MEMORY_INFO, IDXGIAdapter, IDXGIAdapter3, IDXGIFactory1,
};
use windows::core::Interface;

/// PCI vendor ID for NVIDIA Corporation.
///
/// Used to filter `EnumAdapters1` results so `device_index` counts only
/// NVIDIA adapters, matching `NVML`'s enumeration view.
const NVIDIA_VENDOR_ID: u32 = 0x10DE;

/// PCI vendor ID for the Microsoft Basic Render Driver.
///
/// `MSBR` is the synthetic adapter every Windows install ships with — it
/// has no underlying GPU memory. Skipped during the
/// [`enumerate_non_nvidia`] walk so it never surfaces in `Snapshot::all()`.
const MICROSOFT_BASIC_VENDOR_ID: u32 = 0x1414;

/// Combined result of a single `DXGI` query for a given (NVIDIA-filtered) adapter index.
///
/// Returned by [`query`].
pub(super) struct DxgiQueryResult {
    /// Per-process VRAM usage in bytes (`CurrentUsage` from `DXGI_QUERY_VIDEO_MEMORY_INFO`).
    /// This is the calling process's own GPU memory consumption — DXGI is
    /// WDDM-aware, so this number is reliable under Windows.
    pub current_usage: u64,
    /// Total dedicated VRAM on the adapter in bytes
    /// (`DedicatedVideoMemory` from `DXGI_ADAPTER_DESC`).
    pub dedicated_video_memory: u64,
    /// Adapter name parsed from the UTF-16 `DXGI_ADAPTER_DESC.Description` field.
    /// `None` when the name is empty after trimming trailing nulls.
    pub adapter_name: Option<String>,
}

/// Walk every `DXGI` adapter in raw `EnumAdapters1` order, calling `visit`
/// with each adapter's raw index, its `IDXGIAdapter` and its
/// `DXGI_ADAPTER_DESC`, until `visit` breaks or the adapters run out.
///
/// The one `EnumAdapters1` loop in this module, and so the one place its
/// adapter-skipping policy lives: an adapter whose `IDXGIAdapter` cast or
/// `GetDesc` fails is skipped (with a `debug-output` trace) rather than
/// ending the walk — `EnumAdapters1` itself failing is the only
/// end-of-walk signal. Every lookup built on this walker therefore agrees
/// on which adapters exist and how NVIDIA indices are numbered.
///
/// A visitor rather than an iterator on purpose: `CONVENTIONS.md`
/// (Pattern 3) forbids carrying COM pointers across function boundaries,
/// and a visitor only ever borrows each adapter inside this function's
/// frame.
///
/// Returns `None` if the `DXGI` factory cannot be created;
/// `Some(ControlFlow::Break(b))` if `visit` stopped the walk with `b`;
/// `Some(ControlFlow::Continue(()))` if every adapter was visited.
#[allow(unsafe_code)]
fn walk_adapters<B>(
    mut visit: impl FnMut(u32, &IDXGIAdapter, &DXGI_ADAPTER_DESC) -> ControlFlow<B>,
) -> Option<ControlFlow<B>> {
    // SAFETY: CreateDXGIFactory1 is a documented COM factory function.
    // It initializes COM internally if needed; the returned IDXGIFactory1
    // is reference-counted and released when `factory` is dropped.
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;

    let mut raw_idx: u32 = 0;
    loop {
        // SAFETY: EnumAdapters1 returns Err once raw_idx is past the last
        // enumerated adapter; that is the natural end of the walk.
        let Ok(adapter1) = (unsafe { factory.EnumAdapters1(raw_idx) }) else {
            return Some(ControlFlow::Continue(()));
        };
        let Ok(adapter) = adapter1.cast::<IDXGIAdapter>() else {
            // A single malformed/unexpected adapter must not abort the
            // whole walk — skip it and keep walking. raw_idx must still
            // advance, or this would spin forever re-probing the same
            // failing index.
            #[cfg(feature = "debug-output")]
            eprintln!("[DXGI debug] adapter[raw#{raw_idx}]: IDXGIAdapter cast failed, skipping");
            raw_idx += 1;
            continue;
        };
        // SAFETY: GetDesc fills DXGI_ADAPTER_DESC. The adapter handle is
        // valid (just acquired above with EnumAdapters1 returning S_OK).
        let Ok(desc) = (unsafe { adapter.GetDesc() }) else {
            // Same reasoning as the cast failure above: skip, don't abort.
            #[cfg(feature = "debug-output")]
            eprintln!("[DXGI debug] adapter[raw#{raw_idx}]: GetDesc failed, skipping");
            raw_idx += 1;
            continue;
        };
        if let ControlFlow::Break(b) = visit(raw_idx, &adapter, &desc) {
            return Some(ControlFlow::Break(b));
        }
        raw_idx += 1;
    }
}

/// Visit every `DXGI` adapter, never stopping early. `None` if the
/// `DXGI` factory cannot be created. A thin wrapper over
/// [`walk_adapters`] for the exhaustive walks ([`enumerate_non_nvidia`],
/// [`device_count`]).
fn for_each_adapter(mut visit: impl FnMut(u32, &IDXGIAdapter, &DXGI_ADAPTER_DESC)) -> Option<()> {
    walk_adapters(|raw_idx, adapter, desc| {
        visit(raw_idx, adapter, desc);
        ControlFlow::<Infallible>::Continue(())
    })
    .map(|_| ())
}

/// Apply `f` to the `idx`-th adapter that passes [`is_nvidia_dgpu`] — the
/// indexing every per-device lookup in this module shares, matching
/// `NVML`'s enumeration view — and return its result. `None` if the
/// `DXGI` factory cannot be created, if fewer than `idx + 1` NVIDIA
/// adapters exist, or if `f` itself returns `None`.
///
/// `f` receives the adapter's raw `EnumAdapters1` index, used only by
/// `debug-output` traces.
fn nth_nvidia_adapter<T>(
    idx: u32,
    mut f: impl FnMut(u32, &IDXGIAdapter, &DXGI_ADAPTER_DESC) -> Option<T>,
) -> Option<T> {
    let mut nvidia_idx: u32 = 0;
    walk_adapters(|raw_idx, adapter, desc| {
        if !is_nvidia_dgpu(desc) {
            return ControlFlow::Continue(());
        }
        if nvidia_idx == idx {
            return ControlFlow::Break(f(raw_idx, adapter, desc));
        }
        nvidia_idx += 1;
        ControlFlow::Continue(())
    })?
    .break_value()
    .flatten()
}

/// Whether `desc` describes an NVIDIA adapter with dedicated `VRAM`
/// (`VendorId == 0x10DE` and `DedicatedVideoMemory > 0`) — the filter that
/// defines the `device_index` space [`device_count`] counts and every
/// per-index lookup ([`query`], `adapter_name`, `adapter_luid`,
/// `adapter_dedicated_video_memory`) indexes into.
const fn is_nvidia_dgpu(desc: &DXGI_ADAPTER_DESC) -> bool {
    desc.VendorId == NVIDIA_VENDOR_ID && desc.DedicatedVideoMemory > 0
}

/// Adapter name from `DXGI_ADAPTER_DESC.Description`, trailing `NUL`s
/// trimmed; `None` when nothing remains.
fn description(desc: &DXGI_ADAPTER_DESC) -> Option<String> {
    // BORROW: explicit String::from_utf16_lossy + trim_end_matches +
    // to_owned — desc.Description is fixed-size [u16; 128]; we trim
    // trailing NULs and need an owned String to return.
    let raw_name = String::from_utf16_lossy(&desc.Description);
    let trimmed = raw_name.trim_end_matches('\0');
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// A `DXGI_ADAPTER_DESC` memory size (`usize`) as `u64` bytes.
const fn bytes(size: usize) -> u64 {
    // CAST: usize → u64, DedicatedVideoMemory / SharedSystemMemory are
    // usize in DXGI_ADAPTER_DESC; lossless on every supported target
    // (usize is at most 64 bits).
    #[allow(clippy::as_conversions)]
    let bytes = size as u64;
    bytes
}

/// Run a single `DXGI` query for the (NVIDIA-filtered) adapter at `idx`.
///
/// Finds the `idx`-th NVIDIA adapter with non-zero dedicated VRAM, then
/// casts to `IDXGIAdapter3` and queries `DXGI_MEMORY_SEGMENT_GROUP_LOCAL`.
/// Returns `None` if the `DXGI` factory cannot be created, if `idx` is past
/// the count of qualifying adapters, or if the `IDXGIAdapter3` cast or the
/// memory query fails on that adapter.
#[allow(unsafe_code)]
pub(super) fn query(idx: u32) -> Option<DxgiQueryResult> {
    nth_nvidia_adapter(idx, |raw_idx, adapter, desc| {
        let total = bytes(desc.DedicatedVideoMemory);
        let adapter_name = description(desc);

        let adapter3: IDXGIAdapter3 = adapter.cast().ok()?;
        let mut mem_info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
        // SAFETY: QueryVideoMemoryInfo fills DXGI_QUERY_VIDEO_MEMORY_INFO.
        // Node 0 = primary GPU node. DXGI_MEMORY_SEGMENT_GROUP_LOCAL =
        // dedicated VRAM segment on discrete GPUs.
        unsafe {
            adapter3.QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &raw mut mem_info)
        }
        .ok()?;

        #[cfg(feature = "debug-output")]
        eprintln!(
            "[DXGI debug] adapter[nvidia#{idx} raw#{raw_idx}]: name={adapter_name:?}, \
             dedicated_vram={total}, current_usage={}, budget={}",
            mem_info.CurrentUsage, mem_info.Budget
        );
        // EXPLICIT: raw_idx is read only by the debug-output trace above.
        #[cfg(not(feature = "debug-output"))]
        let _ = raw_idx;

        Some(DxgiQueryResult {
            current_usage: mem_info.CurrentUsage,
            dedicated_video_memory: total,
            adapter_name,
        })
    })
}

/// Lightweight adapter-name lookup that skips `QueryVideoMemoryInfo`.
///
/// Used by the `device_info` dispatcher to augment `NVML`'s name with
/// the friendlier `DXGI` `Description` string without paying for a full
/// `query`. Returns `None` if the `DXGI` factory cannot be created, if
/// `idx` is past the NVIDIA-adapter count, or if the adapter's
/// description is empty.
///
/// Gated `cfg(feature = "nvml")` to match the sole call site at
/// [`crate::gpu::device_info`], which only invokes this helper when
/// `NVML` produced the primary numerics. Without `nvml` the function
/// would be dead code (relevant only on the unusual `pdh + dxgi`
/// without `nvml` feature combination).
#[cfg(feature = "nvml")]
pub(super) fn adapter_name(idx: u32) -> Option<String> {
    nth_nvidia_adapter(idx, |_, _, desc| description(desc))
}

/// `LUID` of the `idx`-th NVIDIA-filtered adapter, as `(high, low)`.
///
/// Uses the same NVIDIA-filter rule as [`query`] (vendor ID `0x10DE` +
/// non-zero `DedicatedVideoMemory`) and returns the `(HighPart, LowPart)`
/// pair of the adapter's `LUID` from `DXGI_ADAPTER_DESC.AdapterLuid`.
/// Returns `None` if `idx` is past the count of qualifying adapters, or if
/// the `DXGI` factory cannot be created.
///
/// Consumed by [`crate::gpu::pdh`] to filter PDH counter instances of
/// the form `pid_NNNN_luid_0xHHHHHHHH_0xHHHHHHHH_phys_N` down to those
/// belonging to the target adapter. The `LUID` is the stable kernel-side
/// identifier of a `WDDM` adapter — preserved across reboots until
/// driver-level reconfiguration.
///
/// Gated `cfg(feature = "pdh")` to match its sole call site. Without
/// `pdh` the function is dead code (relevant only on the unusual
/// `dxgi` without `pdh` feature combination).
#[cfg(feature = "pdh")]
pub(super) fn adapter_luid(idx: u32) -> Option<(i32, u32)> {
    nth_nvidia_adapter(idx, |_, _, desc| {
        Some((desc.AdapterLuid.HighPart, desc.AdapterLuid.LowPart))
    })
}

/// Dedicated `VRAM` capacity in bytes of the `idx`-th NVIDIA-filtered
/// adapter (`DXGI_ADAPTER_DESC.DedicatedVideoMemory`).
///
/// Uses the same NVIDIA-filter rule as [`adapter_luid`]. Returns `None` if
/// `idx` is past the count of qualifying adapters, or if the `DXGI`
/// factory cannot be created.
///
/// Consumed by `crate::gpu::pdh`'s adapter-wide spill query
/// (v0.2.5) as the dedicated-capacity figure: the `PDH`
/// `GPU Adapter Memory` counter set carries no `Dedicated Limit`
/// counter (verified live 2026-07-22), so the static `DXGI` capacity
/// stands in. Note this is the `DXGI` nominal figure — on the
/// reference card it differs slightly from `NVML`'s `total` (see
/// `docs/roadmap-v0.2.4.md` for the three-source capacity
/// comparison); for the 85% saturation threshold
/// (`DEFAULT_DEDICATED_THRESHOLD_PCT` in `crate::spill`) the
/// difference is immaterial.
///
/// Gated `cfg(feature = "pdh")` to match its sole call site, like
/// [`adapter_luid`].
#[cfg(feature = "pdh")]
pub(super) fn adapter_dedicated_video_memory(idx: u32) -> Option<u64> {
    nth_nvidia_adapter(idx, |_, _, desc| Some(bytes(desc.DedicatedVideoMemory)))
}

/// One non-NVIDIA, non-`MSBR` `DXGI` adapter that exposes some form of GPU memory.
///
/// Returned by [`enumerate_non_nvidia`] for `Snapshot::all()` on Windows
/// to surface AMD / Intel iGPUs alongside the NVIDIA dGPU(s) `NVML`
/// already enumerates. NVIDIA adapters are intentionally excluded from
/// this enumeration — `NVML` is the authoritative source for them
/// (correct device-wide totals, driver-side `free` figure that accounts
/// for reservation / alignment).
pub(super) struct DxgiAdapterEntry {
    /// Adapter name parsed from `DXGI_ADAPTER_DESC.Description`.
    /// `None` when the description is empty after trimming trailing nulls.
    pub adapter_name: Option<String>,
    /// Per-process VRAM usage in bytes — `CurrentUsage` of the
    /// `DXGI_MEMORY_SEGMENT_GROUP_LOCAL` segment. Zero when the adapter
    /// does not expose `IDXGIAdapter3` or `QueryVideoMemoryInfo` fails;
    /// not an error condition (best-effort).
    pub current_usage: u64,
    /// `DXGI_ADAPTER_DESC.DedicatedVideoMemory` — dedicated `VRAM` in
    /// bytes. Non-zero on dGPUs and on iGPUs with BIOS-allocated UMA
    /// chunks; zero on iGPUs without UMA.
    pub dedicated_video_memory: u64,
    /// `DXGI_ADAPTER_DESC.SharedSystemMemory` — `WDDM` shared-memory
    /// budget in bytes (the OS-managed slice of system RAM the GPU may
    /// commit). Non-zero on every real adapter; useful as the
    /// `total_bytes` fallback for iGPUs without a dedicated chunk.
    pub shared_system_memory: u64,
}

/// Walk every `DXGI` adapter, returning one entry per non-NVIDIA, non-`MSBR`
/// adapter that exposes any GPU-accessible memory.
///
/// Filter rule: `VendorId != 0x10DE` (NVIDIA, handled by `NVML`) AND
/// `VendorId != 0x1414` (`MSBR`) AND
/// (`DedicatedVideoMemory > 0` OR `SharedSystemMemory > 0`).
///
/// Used by `Snapshot::all()`. Returns an empty `Vec` if `DXGI` itself
/// fails to load, or the system has no qualifying non-NVIDIA adapters.
#[allow(unsafe_code)]
#[must_use]
pub(super) fn enumerate_non_nvidia() -> Vec<DxgiAdapterEntry> {
    let mut out: Vec<DxgiAdapterEntry> = Vec::new();
    // EXPLICIT: a factory failure visits nothing and leaves `out` empty,
    // which is exactly this function's documented "DXGI failed to load"
    // return — so the walk's Option carries nothing to act on here.
    let _ = for_each_adapter(|raw_idx, adapter, desc| {
        let dedicated = bytes(desc.DedicatedVideoMemory);
        let shared = bytes(desc.SharedSystemMemory);
        let qualifies = desc.VendorId != NVIDIA_VENDOR_ID
            && desc.VendorId != MICROSOFT_BASIC_VENDOR_ID
            && (dedicated > 0 || shared > 0);
        if !qualifies {
            return;
        }
        let adapter_name = description(desc);

        // Per-process LOCAL CurrentUsage. Best-effort: an adapter
        // that doesn't implement IDXGIAdapter3, or where the query
        // fails, contributes 0 here. The dispatcher records the
        // 0 as a per-process reading, not as an error.
        let current_usage = adapter
            .cast::<IDXGIAdapter3>()
            .ok()
            .and_then(|a3| {
                let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                // SAFETY: QueryVideoMemoryInfo fills the caller-provided
                // DXGI_QUERY_VIDEO_MEMORY_INFO. Node 0 = primary GPU
                // node; LOCAL = dedicated VRAM segment.
                unsafe {
                    a3.QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &raw mut info)
                }
                .ok()
                .map(|()| info.CurrentUsage)
            })
            .unwrap_or(0);

        #[cfg(feature = "debug-output")]
        eprintln!(
            "[DXGI debug] non-nvidia adapter[raw#{raw_idx}]: vendor={:#x}, \
             name={adapter_name:?}, dedicated={dedicated}, shared={shared}, \
             current_usage={current_usage}",
            desc.VendorId
        );
        // EXPLICIT: raw_idx is read only by the debug-output trace above.
        #[cfg(not(feature = "debug-output"))]
        let _ = raw_idx;

        out.push(DxgiAdapterEntry {
            adapter_name,
            current_usage,
            dedicated_video_memory: dedicated,
            shared_system_memory: shared,
        });
    });
    out
}

/// Number of NVIDIA adapters with non-zero dedicated VRAM visible to `DXGI`.
///
/// Used as the `device_count` fallback when `NVML` is unavailable, and
/// for bounds-checking `idx` on Windows in `device_info` /
/// `process_gpu_info` when both `NVML` and `nvidia-smi` are absent.
/// `None` if the `DXGI` factory cannot be created.
pub(super) fn device_count() -> Option<u32> {
    let mut count: u32 = 0;
    for_each_adapter(|_, _, desc| {
        if is_nvidia_dgpu(desc) {
            count += 1;
        }
    })?;

    #[cfg(feature = "debug-output")]
    eprintln!("[DXGI debug] device_count = {count} (NVIDIA adapters with non-zero VRAM)");

    Some(count)
}
