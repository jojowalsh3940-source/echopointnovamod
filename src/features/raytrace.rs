//! CPU-side line-of-sight test via UE's BP-exposed `LineTraceSingle`.
//!
//! UE's CPU cull renders skeletal meshes whenever their bounds pass the
//! frustum + hardware occlusion query, so `LastRenderTimeOnScreen` will tick
//! through walls. To get a real "visible to the player" answer we have to ask
//! the engine to trace a ray, which means calling
//! `UKismetSystemLibrary::LineTraceSingle` via `UObject::ProcessEvent`.
//!
//! Parameter layout and FHitResult offsets come from the dumped SDK
//! (`CppSDK/SDK/Engine_parameters.hpp`, `Engine_structs.hpp`).

use std::sync::{Mutex, OnceLock};

use crate::features::filter;
use crate::memory;

const PROCESS_EVENT_VTABLE_IDX: usize = 0x44; // 68
const TRACE_PARAMS_SIZE: usize = 0xF0;
const TRACE_CHANNEL_VISIBILITY: u8 = 0; // TraceTypeQuery1 — Visibility in default UE setup

struct RaytraceState {
    line_trace_fn: usize, // UFunction*
    initialized: bool,
}

static STATE: OnceLock<Mutex<RaytraceState>> = OnceLock::new();

fn state() -> &'static Mutex<RaytraceState> {
    STATE.get_or_init(|| Mutex::new(RaytraceState {
        line_trace_fn: 0,
        initialized: false,
    }))
}

pub fn ensure_initialized(module_base: usize) -> bool {
    let mut s = match state().lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    if s.initialized {
        return s.line_trace_fn != 0;
    }
    let fn_ptr = filter::find_class_by_name(
        module_base,
        memory::GOBJECTS_OFFSET,
        "LineTraceSingle",
    ).unwrap_or(0);
    s.line_trace_fn = fn_ptr;
    s.initialized = true;
    fn_ptr != 0
}

pub fn line_trace_fn() -> usize {
    state().lock().map(|s| s.line_trace_fn).unwrap_or(0)
}

/// Returns true if the line from `start` to `end` is unobstructed once
/// `ignore` (typically `[player_pawn, enemy]`) is excluded.
///
/// Returns true (visible) on any failure — better to draw an enemy we can't
/// verify than to silently hide a real threat.
pub fn line_of_sight(
    world: usize,
    start: [f32; 3],
    end: [f32; 3],
    ignore: &[usize],
) -> bool {
    let func = line_trace_fn();
    if world == 0 || func == 0 {
        return true;
    }

    let mut params = [0u8; TRACE_PARAMS_SIZE];
    let p = params.as_mut_ptr();

    unsafe {
        // 0x00: WorldContextObject
        *(p.add(0x00) as *mut usize) = world;
        // 0x08: Start (FVector)
        *(p.add(0x08) as *mut [f32; 3]) = start;
        // 0x14: End (FVector)
        *(p.add(0x14) as *mut [f32; 3]) = end;
        // 0x20: TraceChannel
        *(p.add(0x20)) = TRACE_CHANNEL_VISIBILITY;
        // 0x21: bTraceComplex = false (simple collision is enough for line of sight)
        *(p.add(0x21)) = 0;
        // 0x28: ActorsToIgnore TArray — point Data at the caller's slice
        *(p.add(0x28) as *mut *const usize) = ignore.as_ptr();
        *(p.add(0x30) as *mut i32) = ignore.len() as i32; // Num
        *(p.add(0x34) as *mut i32) = ignore.len() as i32; // Max
        // 0x38: DrawDebugType = None
        *(p.add(0x38)) = 0;
        // 0x3C..0xC4: FHitResult — zeroed
        // 0xC4: bIgnoreSelf = true
        *(p.add(0xC4)) = 1;
        // 0xC8..0xE8: TraceColor / TraceHitColor — zeroed
        // 0xE8: DrawTime = 0
        // 0xEC: ReturnValue (out)

        // ProcessEvent through UObject vtable[68]
        let vtable = *(world as *const usize);
        if vtable == 0 {
            return true;
        }
        let pe_addr = *(vtable as *const usize).add(PROCESS_EVENT_VTABLE_IDX);
        if pe_addr == 0 {
            return true;
        }
        type ProcessEventFn = unsafe extern "system" fn(usize, usize, *mut u8);
        let pe: ProcessEventFn = std::mem::transmute(pe_addr);
        pe(world, func, p);

        // ReturnValue at 0xEC: true = blocking hit (wall between us), so visible = NOT blocked
        *(p.add(0xEC)) == 0
    }
}
