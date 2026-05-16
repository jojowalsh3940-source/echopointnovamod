use std::sync::Mutex;
use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Memory::{
    VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS,
};

pub const GWORLD_OFFSET: usize = 0x58BE190;
pub const GOBJECTS_OFFSET: usize = 0x5777010;

pub const PERSISTENT_LEVEL_OFFSET: usize = 0x30;
pub const ACTORS_ARRAY_OFFSET: usize = 0x98;

pub const ACTOR_ROOT_COMPONENT_OFFSET: usize = 0x130;
pub const COMPONENT_LOCATION_OFFSET: usize = 0x11C;

pub const GAME_STATE_OFFSET: usize = 0x120;
pub const GAMESTATE_DEBUG_GODMODE_OFFSET: usize = 0x34A;

pub const GAME_INSTANCE_OFFSET: usize = 0x180;
pub const LOCAL_PLAYERS_OFFSET: usize = 0x38;
pub const PLAYER_CONTROLLER_OFFSET: usize = 0x30;
pub const CAMERA_MANAGER_OFFSET: usize = 0x2B8;
pub const CAMERA_CACHE_OFFSET: usize = 0x290;
pub const CAMERA_CACHE_PRIVATE_OFFSET: usize = 0x1AE0;
pub const VIEW_TARGET_OFFSET: usize = 0xE90;
pub const CAMERA_CACHE_POV_OFFSET: usize = 0x10;
pub const UOBJECT_CLASS_OFFSET: usize = 0x10;
pub const UOBJECT_NAME_OFFSET: usize = 0x18;
pub const CONTROLLER_PAWN_OFFSET: usize = 0x250;
pub const APPEND_STRING_OFFSET: usize = 0x01A6C410;

// AGLBaseCharacter (mechs: BP_Harrier, BP_RoverBase, BP_RoverSpider, ...)
pub const MECH_HP_OFFSET: usize = 0x564;        // float MyHP
pub const MECH_IS_DEAD_OFFSET: usize = 0x56D;   // bool bIsDead

// AHuman (humans: BP_Human_Enemy_*, BP_GLHuman_*)
pub const HUMAN_HP_OFFSET: usize = 0xDEC;       // int32 MyHP

// AALSBaseCharacter::MovementState — uint8 enum. The Ragdoll value (4) fires the
// moment a human starts dying/ragdolling, well before MyHP settles at 0, so we
// use it as the fast death signal for the AHuman chain.
pub const ALS_MOVEMENT_STATE_OFFSET: usize = 0x5C8;
pub const ALS_MOVEMENT_STATE_RAGDOLL: u8 = 4;

// UCapsuleComponent — ACharacter::RootComponent in both chains. Unscaled fields,
// but enemies in this game use ComponentScale 1.0 so no scaling needed for ESP.
pub const CAPSULE_HALF_HEIGHT_OFFSET: usize = 0x468;
pub const CAPSULE_RADIUS_OFFSET: usize = 0x46C;

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

const REGION_CACHE_SIZE: usize = 32;

#[derive(Clone, Copy)]
struct ValidRegion {
    base: usize,
    end: usize,
}

struct RegionCache {
    entries: [ValidRegion; REGION_CACHE_SIZE],
    used: usize,
    next_slot: usize,
}

impl RegionCache {
    const fn new() -> Self {
        Self {
            entries: [ValidRegion { base: 0, end: 0 }; REGION_CACHE_SIZE],
            used: 0,
            next_slot: 0,
        }
    }

    fn clear(&mut self) {
        self.used = 0;
        self.next_slot = 0;
    }

    fn contains(&self, addr: usize, end: usize) -> bool {
        for i in 0..self.used {
            let r = self.entries[i];
            if addr >= r.base && end <= r.end {
                return true;
            }
        }
        false
    }

    fn insert(&mut self, base: usize, end: usize) {
        self.entries[self.next_slot] = ValidRegion { base, end };
        self.next_slot = (self.next_slot + 1) % REGION_CACHE_SIZE;
        if self.used < REGION_CACHE_SIZE {
            self.used += 1;
        }
    }
}

static REGION_CACHE: Mutex<RegionCache> = Mutex::new(RegionCache::new());

pub fn clear_region_cache() {
    if let Ok(mut c) = REGION_CACHE.lock() {
        c.clear();
    }
}

fn is_readable(addr: usize, size: usize) -> bool {
    if addr < 0x10000 || addr > 0x7FFF_FFFF_FFFF {
        return false;
    }
    let end = addr.saturating_add(size);

    if let Ok(cache) = REGION_CACHE.lock() {
        if cache.contains(addr, end) {
            return true;
        }
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
        let region_base = mbi.BaseAddress as usize;
        let region_end = region_base.saturating_add(mbi.RegionSize);

        if let Ok(mut c) = REGION_CACHE.lock() {
            c.insert(region_base, region_end);
        }

        end <= region_end
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

pub fn actor_slice(array: &ActorArray) -> &[usize] {
    if array.data == 0 || array.count <= 0 {
        return &[];
    }
    let bytes = (array.count as usize).saturating_mul(8);
    if !is_readable(array.data, bytes) {
        return &[];
    }
    unsafe {
        std::slice::from_raw_parts(array.data as *const usize, array.count as usize)
    }
}

pub fn get_actor_location(actor: usize) -> Option<[f32; 3]> {
    if actor == 0 { return None; }
    let root = safe_read_ptr(actor + ACTOR_ROOT_COMPONENT_OFFSET);
    if root == 0 { return None; }
    safe_read_vec3(root + COMPONENT_LOCATION_OFFSET)
}

/// Returns (full_height_cm, full_width_cm) read from the actor's RootComponent
/// when it is a UCapsuleComponent (true for both Human and Mech chains).
/// None when the values are missing or implausible.
pub fn get_actor_capsule(actor: usize) -> Option<(f32, f32)> {
    if actor == 0 { return None; }
    let root = safe_read_ptr(actor + ACTOR_ROOT_COMPONENT_OFFSET);
    if root == 0 { return None; }
    let half_h = safe_read_f32(root + CAPSULE_HALF_HEIGHT_OFFSET)?;
    let radius = safe_read_f32(root + CAPSULE_RADIUS_OFFSET)?;
    if !(5.0..=2000.0).contains(&half_h) || !(5.0..=2000.0).contains(&radius) {
        return None;
    }
    Some((half_h * 2.0, radius * 2.0))
}

pub fn get_actor_class(actor: usize) -> usize {
    if actor == 0 { return 0; }
    safe_read_ptr(actor + UOBJECT_CLASS_OFFSET)
}

pub fn get_player_pawn(pc: usize) -> usize {
    if pc == 0 { return 0; }
    safe_read_ptr(pc + CONTROLLER_PAWN_OFFSET)
}

pub fn read_i32_at(addr: usize) -> Option<i32> {
    if !is_readable(addr, 4) { return None; }
    Some(unsafe { *(addr as *const i32) })
}

pub fn read_ptr_at(addr: usize) -> usize {
    safe_read_ptr(addr)
}

pub fn read_u8_at(addr: usize) -> Option<u8> {
    if !is_readable(addr, 1) { return None; }
    Some(unsafe { *(addr as *const u8) })
}

pub fn write_u8_at(addr: usize, value: u8) -> bool {
    if !is_readable(addr, 1) { return false; }
    unsafe { *(addr as *mut u8) = value; }
    true
}

pub fn get_game_state(world: usize) -> usize {
    if world == 0 { return 0; }
    safe_read_ptr(world + GAME_STATE_OFFSET)
}

#[repr(C)]
struct FString {
    data: *mut u16,
    num: i32,
    max: i32,
}

pub fn resolve_fname(module_base: usize, fname_addr: usize) -> Option<String> {
    if module_base == 0 || fname_addr < 0x10000 { return None; }
    if !is_readable(fname_addr, 8) { return None; }

    let fn_addr = module_base + APPEND_STRING_OFFSET;
    if !is_readable(fn_addr, 16) { return None; }

    type AppendStringFn = unsafe extern "C" fn(*const u8, *mut FString);
    let f: AppendStringFn = unsafe { std::mem::transmute(fn_addr) };

    let mut buffer: [u16; 1024] = [0; 1024];
    let mut fstring = FString {
        data: buffer.as_mut_ptr(),
        num: 0,
        max: 1024,
    };

    unsafe { f(fname_addr as *const u8, &mut fstring); }

    let raw_len = fstring.num;
    if raw_len <= 0 || raw_len > 1024 { return None; }
    let mut len = raw_len as usize;
    if len > 0 && unsafe { *fstring.data.add(len - 1) } == 0 { len -= 1; }
    if len == 0 { return None; }

    let slice = unsafe { std::slice::from_raw_parts(fstring.data, len) };
    Some(String::from_utf16_lossy(slice))
}

pub fn get_class_name(module_base: usize, class_ptr: usize) -> Option<String> {
    if class_ptr == 0 { return None; }
    resolve_fname(module_base, class_ptr + UOBJECT_NAME_OFFSET)
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    #[default]
    None,
    Human,
    Mech,
}

pub fn is_actor_alive(actor: usize, kind: EnemyKind) -> bool {
    if actor == 0 { return true; }
    match kind {
        EnemyKind::None => true,
        EnemyKind::Human => {
            let ms_addr = actor + ALS_MOVEMENT_STATE_OFFSET;
            if is_readable(ms_addr, 1) {
                let state = unsafe { *(ms_addr as *const u8) };
                if state == ALS_MOVEMENT_STATE_RAGDOLL {
                    return false;
                }
            }
            let addr = actor + HUMAN_HP_OFFSET;
            if is_readable(addr, 4) {
                let hp = unsafe { *(addr as *const i32) };
                if (0..=100_000).contains(&hp) {
                    return hp > 0;
                }
            }
            true
        }
        EnemyKind::Mech => {
            let dead_addr = actor + MECH_IS_DEAD_OFFSET;
            if is_readable(dead_addr, 1) {
                let dead = unsafe { *(dead_addr as *const u8) };
                if dead == 1 { return false; }
            }
            if let Some(hp) = safe_read_f32(actor + MECH_HP_OFFSET) {
                if hp >= 0.0 && hp <= 100_000.0 {
                    return hp > 0.0;
                }
            }
            true
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct ClassGroup {
    pub class_ptr: usize,
    pub count: i32,
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
