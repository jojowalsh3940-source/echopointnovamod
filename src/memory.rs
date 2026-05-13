use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Memory::{
    VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS,
};

pub const GWORLD_OFFSET: usize = 0x58BE190;

pub const PERSISTENT_LEVEL_OFFSET: usize = 0x30;
pub const ACTORS_ARRAY_OFFSET: usize = 0x98;

pub const ACTOR_ROOT_COMPONENT_OFFSET: usize = 0x130;
pub const COMPONENT_LOCATION_OFFSET: usize = 0x11C;

pub const GAME_INSTANCE_OFFSET: usize = 0x180;
pub const LOCAL_PLAYERS_OFFSET: usize = 0x38;
pub const PLAYER_CONTROLLER_OFFSET: usize = 0x30;
pub const CAMERA_MANAGER_OFFSET: usize = 0x2B8;
pub const CAMERA_CACHE_OFFSET: usize = 0x290;
pub const CAMERA_CACHE_PRIVATE_OFFSET: usize = 0x1AE0;
pub const VIEW_TARGET_OFFSET: usize = 0xE90;
pub const CAMERA_CACHE_POV_OFFSET: usize = 0x10;
pub const UOBJECT_CLASS_OFFSET: usize = 0x10;
pub const CONTROLLER_PAWN_OFFSET: usize = 0x250;

pub const CLASS_GROUP_COUNT: usize = 64;
pub const SELECTED_CLASS_COUNT: usize = 8;

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

pub fn get_actor_class(actor: usize) -> usize {
    if actor == 0 { return 0; }
    safe_read_ptr(actor + UOBJECT_CLASS_OFFSET)
}

pub fn get_player_pawn_class(pc: usize) -> usize {
    if pc == 0 { return 0; }
    let pawn = safe_read_ptr(pc + CONTROLLER_PAWN_OFFSET);
    get_actor_class(pawn)
}

#[derive(Default, Clone, Copy)]
pub struct ClassGroup {
    pub class_ptr: usize,
    pub count: i32,
    pub sample_loc: [f32; 3],
}

#[derive(Default, Clone, Copy)]
pub struct PovSample {
    pub location: [f32; 3],
    pub rotation: [f32; 3],
    pub fov: f32,
    pub valid: bool,
}

fn read_pov(addr: usize) -> PovSample {
    let mut s = PovSample::default();
    if let Some(loc) = safe_read_vec3(addr) {
        if let Some(rot) = safe_read_vec3(addr + 0xC) {
            if let Some(fov) = safe_read_f32(addr + 0x18) {
                if fov >= 1.0 && fov <= 179.0 {
                    s.location = loc;
                    s.rotation = rot;
                    s.fov = fov;
                    s.valid = true;
                }
            }
        }
    }
    s
}

#[derive(Default)]
pub struct CameraChain {
    pub gi: usize,
    pub lp_array: usize,
    pub local_player: usize,
    pub pc: usize,
    pub cm: usize,
    pub location: [f32; 3],
    pub rotation: [f32; 3],
    pub fov: f32,
    pub ok: bool,
    pub source: u8,
    pub pov_public: PovSample,
    pub pov_private: PovSample,
    pub pov_viewtarget: PovSample,
}

pub fn get_camera_chain(world: usize) -> CameraChain {
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
    if c.cm == 0 { return c; }

    c.pov_private    = read_pov(c.cm + CAMERA_CACHE_PRIVATE_OFFSET + CAMERA_CACHE_POV_OFFSET);
    c.pov_viewtarget = read_pov(c.cm + VIEW_TARGET_OFFSET + CAMERA_CACHE_POV_OFFSET);
    c.pov_public     = read_pov(c.cm + CAMERA_CACHE_OFFSET + CAMERA_CACHE_POV_OFFSET);

    let pick = if c.pov_private.valid {
        (c.pov_private, 1)
    } else if c.pov_viewtarget.valid {
        (c.pov_viewtarget, 2)
    } else if c.pov_public.valid {
        (c.pov_public, 3)
    } else {
        return c;
    };

    c.location = pick.0.location;
    c.rotation = pick.0.rotation;
    c.fov = pick.0.fov;
    c.source = pick.1;
    c.ok = true;
    c
}
