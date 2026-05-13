use crate::memory::{
    PovCandidate, PawnCandidate, RotationCandidate,
    POV_CANDIDATE_COUNT, PAWN_CANDIDATE_COUNT, ROT_CANDIDATE_COUNT,
};

pub struct ModState {
    pub esp_enabled: bool,
    pub esp_show_box: bool,
    pub esp_show_distance: bool,
    pub esp_max_distance: f32,
    pub esp_min_distance: f32,
    pub esp_color: [f32; 4],

    pub debug_actor_count: i32,
    pub debug_world_addr: usize,
    pub debug_base_addr: usize,
    pub debug_level_addr: usize,
    pub debug_visible_actors: i32,

    pub debug_gi: usize,
    pub debug_lp_array: usize,
    pub debug_local_player: usize,
    pub debug_pc: usize,
    pub debug_cm: usize,
    pub debug_pov_offset: usize,
    pub debug_pawn_used: usize,
    pub debug_rot_used: usize,
    pub debug_camera_ok: bool,
    pub debug_camera_loc: [f32; 3],
    pub debug_camera_rot: [f32; 3],
    pub debug_camera_fov: f32,
    pub debug_pov_candidates: [PovCandidate; POV_CANDIDATE_COUNT],
    pub debug_pawn_candidates: [PawnCandidate; PAWN_CANDIDATE_COUNT],
    pub debug_rotation_candidates: [RotationCandidate; ROT_CANDIDATE_COUNT],
    pub pawn_motion: [f32; PAWN_CANDIDATE_COUNT],
    pub rotation_motion: [f32; ROT_CANDIDATE_COUNT],

    pub forced_pov_offset: usize,
    pub forced_pawn_offset: usize,
    pub forced_rotation_offset: usize,
}

impl ModState {
    pub const fn new() -> Self {
        Self {
            esp_enabled: true,
            esp_show_box: true,
            esp_show_distance: true,
            esp_max_distance: 200.0,
            esp_min_distance: 2.0,
            esp_color: [1.0, 0.0, 0.0, 1.0],

            debug_actor_count: 0,
            debug_world_addr: 0,
            debug_base_addr: 0,
            debug_level_addr: 0,
            debug_visible_actors: 0,

            debug_gi: 0,
            debug_lp_array: 0,
            debug_local_player: 0,
            debug_pc: 0,
            debug_cm: 0,
            debug_pov_offset: 0,
            debug_pawn_used: 0,
            debug_rot_used: 0,
            debug_camera_ok: false,
            debug_camera_loc: [0.0; 3],
            debug_camera_rot: [0.0; 3],
            debug_camera_fov: 0.0,
            debug_pov_candidates: [PovCandidate {
                offset: 0,
                location: [0.0; 3],
                rotation: [0.0; 3],
                fov: 0.0,
            }; POV_CANDIDATE_COUNT],
            debug_pawn_candidates: [PawnCandidate {
                offset: 0,
                ptr: 0,
                location: [0.0; 3],
            }; PAWN_CANDIDATE_COUNT],
            debug_rotation_candidates: [RotationCandidate {
                offset: 0,
                rotation: [0.0; 3],
            }; ROT_CANDIDATE_COUNT],
            pawn_motion: [0.0; PAWN_CANDIDATE_COUNT],
            rotation_motion: [0.0; ROT_CANDIDATE_COUNT],

            forced_pov_offset: 0,
            forced_pawn_offset: 0,
            forced_rotation_offset: 0,
        }
    }
}
