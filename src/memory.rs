use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Memory::{
    VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS,
};

pub const GWORLD_OFFSET: usize = 0x58BE190;

pub const ACTOR_ROOT_COMPONENT_OFFSET: usize = 0x1A0;
pub const COMPONENT_LOCATION_OFFSET: usize = 0x1D8;

pub static CANDIDATE_LEVEL_OFFSETS: [usize; 3] = [0x30, 0x38, 0x150];
pub static CANDIDATE_ACTORS_OFFSETS: [usize; 4] = [0x98, 0xA0, 0xA8, 0xB0];

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
        if written == 0 {
            return false;
        }
        if mbi.State != MEM_COMMIT {
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
    if !is_readable(addr, 8) {
        return 0;
    }
    unsafe { *(addr as *const usize) }
}

fn safe_read_i32(addr: usize) -> i32 {
    if !is_readable(addr, 4) {
        return 0;
    }
    unsafe { *(addr as *const i32) }
}

fn safe_read_vec3(addr: usize) -> Option<[f32; 3]> {
    if !is_readable(addr, 12) {
        return None;
    }
    unsafe { Some(*(addr as *const [f32; 3])) }
}

pub fn get_gworld(base: usize) -> usize {
    if base == 0 {
        return 0;
    }
    safe_read_ptr(base + GWORLD_OFFSET)
}

pub struct ActorArray {
    pub data: usize,
    pub count: i32,
}

pub fn scan_offsets(world: usize) -> [(usize, usize, i32); 12] {
    let mut results = [(0usize, 0usize, 0i32); 12];
    if world == 0 {
        return results;
    }
    let mut idx = 0;
    for &lv_off in &CANDIDATE_LEVEL_OFFSETS {
        let level = safe_read_ptr(world + lv_off);
        for &arr_off in &CANDIDATE_ACTORS_OFFSETS {
            if idx >= 12 {
                break;
            }
            if level == 0 {
                results[idx] = (lv_off, arr_off, -1);
            } else {
                let count = safe_read_i32(level + arr_off + 8);
                results[idx] = (lv_off, arr_off, count);
            }
            idx += 1;
        }
    }
    results
}

pub fn find_best_actors(world: usize) -> (usize, ActorArray) {
    if world == 0 {
        return (0, ActorArray { data: 0, count: 0 });
    }
    for &lv_off in &CANDIDATE_LEVEL_OFFSETS {
        let level = safe_read_ptr(world + lv_off);
        if level == 0 {
            continue;
        }
        for &arr_off in &CANDIDATE_ACTORS_OFFSETS {
            let data = safe_read_ptr(level + arr_off);
            let count = safe_read_i32(level + arr_off + 8);
            if data != 0 && count > 0 && count < 50_000 {
                return (level, ActorArray { data, count });
            }
        }
    }
    (0, ActorArray { data: 0, count: 0 })
}

pub fn get_actor(array: &ActorArray, index: i32) -> usize {
    if array.data == 0 || index < 0 || index >= array.count {
        return 0;
    }
    safe_read_ptr(array.data + (index as usize * 8))
}

pub fn get_actor_location(actor: usize) -> Option<[f32; 3]> {
    if actor == 0 {
        return None;
    }
    let root = safe_read_ptr(actor + ACTOR_ROOT_COMPONENT_OFFSET);
    if root == 0 {
        return None;
    }
    safe_read_vec3(root + COMPONENT_LOCATION_OFFSET)
}
