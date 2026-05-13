use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Memory::{
    VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS,
};

pub const GWORLD_OFFSET: usize = 0x58BE190;

pub const PERSISTENT_LEVEL_OFFSET: usize = 0x30;
pub const ACTORS_ARRAY_OFFSET: usize = 0x98;

pub const ACTOR_ROOT_COMPONENT_OFFSET: usize = 0x1A0;
pub const COMPONENT_LOCATION_OFFSET: usize = 0x1D8;

pub const GAME_INSTANCE_OFFSET: usize = 0x180;
pub const LOCAL_PLAYERS_OFFSET: usize = 0x38;
pub const PLAYER_CONTROLLER_OFFSET: usize = 0x30;
pub const CAMERA_MANAGER_OFFSET: usize = 0x348;

pub const POV_CANDIDATE_COUNT: usize = 6;
pub const PAWN_CANDIDATE_COUNT: usize = 12;
pub const ROT_CANDIDATE_COUNT: usize = 16;
pub const EYE_HEIGHT_CM: f32 = 64.0;

pub fn get_module_base() -> usize {
    unsafe {
        match GetModuleHandleA(PCSTR::null()) {
            Ok(h) => h.0 as usize,
            Err(_) => 0,
        }
    }
}

fn is_readable(addr: usize, size: usize) -> bool {
    if addr < 0x10000 || addr > 0x7FFF_FFFF_FFFF {
        return false;
    }
    unsafe {
        let mut mbi: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
        let written = VirtualQuery(
            Some(addr as *const _),
            &mut mbi,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        );
        if written == 0 || mbi.State != MEM_COMMIT {
            return false;
        }
        let bad = PAGE_NOACCESS.0 | PAGE_GUARD.0;
        if mbi.Protect.0 & bad != 0 {
            return false;
        }
        let region_end = (mbi.BaseAddress as usize).saturating_add(mbi.RegionSize);
        if addr.saturating_add(size) > region_end {
            return false;
        }
        true
    }
}

fn safe_read_ptr(addr: usize) -> usize {
    if !is_readable(addr, 8) { return 0; }
    unsafe { *(addr as *const usize) }
}

fn safe_read_i32(addr: usize) -> i32 {
    if !is_readable(addr, 4) { return 0; }
    unsafe { *(addr as *const i32) }
}

fn safe_read_f32(addr: usize) -> Option<f32> {
    if !is_readable(addr, 4) { return None; }
    let v = unsafe { *(addr as *const f32) };
    if v.is_finite() { Some(v) } else { None }
}

fn safe_read_vec3(addr: usize) -> Option<[f32; 3]> {
    if !is_readable(addr, 12) { return None; }
    let v = unsafe { *(addr as *const [f32; 3]) };
    if v[0].is_finite() && v[1].is_finite() && v[2].is_finite() {
        Some(v)
    } else {
        None
    }
}

pub fn get_gworld(base: usize) -> usize {
    if base == 0 { return 0; }
    safe_read_ptr(base + GWORLD_OFFSET)
}

pub struct ActorArray {
    pub data: usize,
    pub count: i32,
}

pub fn get_actors(world: usize) -> (usize, ActorArray) {
    if world == 0 {
        return (0, ActorArray { data: 0, count: 0 });
    }
    let level = safe_read_ptr(world + PERSISTENT_LEVEL_OFFSET);
    if level == 0 {
        return (0, ActorArray { data: 0, count: 0 });
    }
    let data = safe_read_ptr(level + ACTORS_ARRAY_OFFSET);
    let count = safe_read_i32(level + ACTORS_ARRAY_OFFSET + 8);
    if data == 0 || count <= 0 || count > 50_000 {
        return (level, ActorArray { data: 0, count: 0 });
    }
    (level, ActorArray { data, count })
}

pub fn get_actor(array: &ActorArray, index: i32) -> usize {
    if array.data == 0 || index < 0 || index >= array.count {
        return 0;
    }
    safe_read_ptr(array.data + (index as usize * 8))
}

pub fn get_actor_location(actor: usize) -> Option<[f32; 3]> {
    if actor == 0 { return None; }
    let root = safe_read_ptr(actor + ACTOR_ROOT_COMPONENT_OFFSET);
    if root == 0 { return None; }
    safe_read_vec3(root + COMPONENT_LOCATION_OFFSET)
}

#[derive(Default, Clone, Copy)]
pub struct PovCandidate {
    pub offset: usize,
    pub location: [f32; 3],
    pub rotation: [f32; 3],
    pub fov: f32,
}

pub fn scan_pov_candidates(cm: usize) -> [PovCandidate; POV_CANDIDATE_COUNT] {
    let mut out: [PovCandidate; POV_CANDIDATE_COUNT] = [PovCandidate::default(); POV_CANDIDATE_COUNT];
    if cm == 0 { return out; }

    let mut found: Vec<(f32, PovCandidate)> = Vec::with_capacity(32);

    let mut off = 0x10usize;
    while off < 0x5000 {
        if let Some(fov) = safe_read_f32(cm + off) {
            if fov >= 30.0 && fov <= 170.0 && fov != 0.0 {
                let loc_off = off.wrapping_sub(0x18);
                let rot_off = off.wrapping_sub(0xC);
                if let (Some(loc), Some(rot)) = (
                    safe_read_vec3(cm + loc_off),
                    safe_read_vec3(cm + rot_off),
                ) {
                    let loc_ok = loc[0].abs() < 1.0e7 && loc[1].abs() < 1.0e7 && loc[2].abs() < 1.0e7;
                    let rot_ok = rot[0].abs() <= 720.0 && rot[1].abs() <= 720.0 && rot[2].abs() <= 720.0;
                    let loc_nonzero = loc[0].abs() + loc[1].abs() + loc[2].abs() > 1.0;
                    if loc_ok && rot_ok && loc_nonzero {
                        let dev = (90.0f32 - fov).abs();
                        let cand = PovCandidate {
                            offset: loc_off,
                            location: loc,
                            rotation: rot,
                            fov,
                        };
                        found.push((dev, cand));
                    }
                }
            }
        }
        off += 4;
    }

    found.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (i, (_, c)) in found.iter().take(POV_CANDIDATE_COUNT).enumerate() {
        out[i] = *c;
    }
    out
}

#[derive(Default, Clone, Copy)]
pub struct PawnCandidate {
    pub offset: usize,
    pub ptr: usize,
    pub location: [f32; 3],
}

#[derive(Default, Clone, Copy)]
pub struct RotationCandidate {
    pub offset: usize,
    pub rotation: [f32; 3],
}

pub fn scan_pawn_candidates(pc: usize, actors: &ActorArray) -> [PawnCandidate; PAWN_CANDIDATE_COUNT] {
    let mut out = [PawnCandidate::default(); PAWN_CANDIDATE_COUNT];
    if pc == 0 { return out; }

    let mut actor_set: Vec<usize> = Vec::new();
    if actors.data != 0 && actors.count > 0 {
        let cap = actors.count.min(50_000) as usize;
        actor_set.reserve(cap);
        for i in 0..actors.count.min(50_000) {
            let a = get_actor(actors, i);
            if a != 0 { actor_set.push(a); }
        }
        actor_set.sort_unstable();
        actor_set.dedup();
    }

    let mut in_array: Vec<PawnCandidate> = Vec::with_capacity(8);
    let mut other: Vec<PawnCandidate> = Vec::with_capacity(8);
    let mut off = 0x10usize;
    while off < 0x800 {
        let ptr = safe_read_ptr(pc + off);
        if ptr != 0 {
            if let Some(loc) = get_actor_location(ptr) {
                if loc[0].abs() + loc[1].abs() + loc[2].abs() > 1.0
                    && loc[0].abs() < 1.0e7 && loc[1].abs() < 1.0e7 && loc[2].abs() < 1.0e7
                {
                    let cand = PawnCandidate { offset: off, ptr, location: loc };
                    if !actor_set.is_empty() && actor_set.binary_search(&ptr).is_ok() {
                        if !in_array.iter().any(|c| c.ptr == ptr) {
                            in_array.push(cand);
                        }
                    } else if !other.iter().any(|c| c.ptr == ptr) {
                        other.push(cand);
                    }
                }
            }
        }
        off += 8;
    }

    let mut all = in_array;
    all.extend(other);
    for (i, c) in all.iter().take(PAWN_CANDIDATE_COUNT).enumerate() {
        out[i] = *c;
    }
    out
}

pub fn scan_rotation_candidates(pc: usize) -> [RotationCandidate; ROT_CANDIDATE_COUNT] {
    let mut out = [RotationCandidate::default(); ROT_CANDIDATE_COUNT];
    if pc == 0 { return out; }

    let mut found: Vec<RotationCandidate> = Vec::with_capacity(32);
    let mut off = 0x10usize;
    while off < 0x800 {
        if let Some(rot) = safe_read_vec3(pc + off) {
            let pitch_ok = rot[0] >= -89.0 && rot[0] <= 89.0;
            let yaw_ok = rot[1].abs() <= 360.0;
            let roll_ok = rot[2].abs() <= 30.0;
            let nonzero = rot[0].abs() + rot[1].abs() + rot[2].abs() > 0.01;
            if pitch_ok && yaw_ok && roll_ok && nonzero {
                found.push(RotationCandidate { offset: off, rotation: rot });
            }
        }
        off += 4;
    }

    for (i, c) in found.iter().take(ROT_CANDIDATE_COUNT).enumerate() {
        out[i] = *c;
    }
    out
}

#[derive(Default)]
pub struct CameraChain {
    pub gi: usize,
    pub lp_array: usize,
    pub local_player: usize,
    pub pc: usize,
    pub cm: usize,
    pub pov_offset: usize,
    pub pawn_used: usize,
    pub rot_used: usize,
    pub location: [f32; 3],
    pub rotation: [f32; 3],
    pub fov: f32,
    pub ok: bool,
    pub candidates: [PovCandidate; POV_CANDIDATE_COUNT],
    pub pawn_candidates: [PawnCandidate; PAWN_CANDIDATE_COUNT],
    pub rotation_candidates: [RotationCandidate; ROT_CANDIDATE_COUNT],
}

pub fn get_camera_chain(
    world: usize,
    actors: &ActorArray,
    forced_pov: usize,
    forced_pawn: usize,
    forced_rotation: usize,
) -> CameraChain {
    let mut c = CameraChain::default();
    if world == 0 { return c; }

    c.gi = safe_read_ptr(world + GAME_INSTANCE_OFFSET);
    if c.gi == 0 { return c; }

    c.lp_array = safe_read_ptr(c.gi + LOCAL_PLAYERS_OFFSET);
    if c.lp_array == 0 { return c; }

    c.local_player = safe_read_ptr(c.lp_array);
    if c.local_player == 0 { return c; }

    c.pc = safe_read_ptr(c.local_player + PLAYER_CONTROLLER_OFFSET);
    if c.pc == 0 { return c; }

    c.cm = safe_read_ptr(c.pc + CAMERA_MANAGER_OFFSET);

    if c.cm != 0 {
        c.candidates = scan_pov_candidates(c.cm);
    }
    c.pawn_candidates = scan_pawn_candidates(c.pc, actors);
    c.rotation_candidates = scan_rotation_candidates(c.pc);

    if forced_pawn != 0 && forced_rotation != 0 {
        let pawn_ptr = safe_read_ptr(c.pc + forced_pawn);
        if let (Some(loc), Some(rot)) = (
            get_actor_location(pawn_ptr),
            safe_read_vec3(c.pc + forced_rotation),
        ) {
            c.location = [loc[0], loc[1], loc[2] + EYE_HEIGHT_CM];
            c.rotation = rot;
            c.fov = 90.0;
            c.pawn_used = forced_pawn;
            c.rot_used = forced_rotation;
            c.ok = true;
            return c;
        }
    }

    if c.cm == 0 { return c; }

    let pov_off = if forced_pov != 0 {
        forced_pov
    } else if c.candidates[0].offset != 0 {
        c.candidates[0].offset
    } else {
        return c;
    };
    c.pov_offset = pov_off;

    let loc = match safe_read_vec3(c.cm + pov_off) {
        Some(v) => v,
        None => return c,
    };
    let rot = match safe_read_vec3(c.cm + pov_off + 0xC) {
        Some(v) => v,
        None => return c,
    };
    let fov = match safe_read_f32(c.cm + pov_off + 0x18) {
        Some(v) => v,
        None => return c,
    };
    if fov < 1.0 || fov > 179.0 {
        return c;
    }

    c.location = loc;
    c.rotation = rot;
    c.fov = fov;
    c.ok = true;
    c
}
