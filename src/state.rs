pub struct ModState {
    pub esp_enabled: bool,
    pub esp_show_box: bool,
    pub esp_show_names: bool,
    pub esp_show_distance: bool,
    pub esp_max_distance: f32,
    pub esp_min_distance: f32,
    pub esp_color_visible: [f32; 4],
    pub esp_color_invisible: [f32; 4],

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
    pub debug_camera_ok: bool,
    pub debug_camera_loc: [f32; 3],
    pub debug_camera_rot: [f32; 3],
    pub debug_camera_fov: f32,
    pub debug_camera_source: u8,
    pub debug_pov_private: [f32; 7],
    pub debug_pov_viewtarget: [f32; 7],
    pub debug_pov_public: [f32; 7],

    pub class_filter_active: bool,
    pub class_groups: [crate::memory::ClassGroup; crate::memory::CLASS_GROUP_COUNT],
    pub selected_classes: [usize; crate::memory::SELECTED_CLASS_COUNT],
    pub debug_player_class: usize,
    pub class_min_count: i32,
    pub class_max_count: i32,
    pub class_search: String,

    pub esp_box_height_cm: f32,
}

impl ModState {
    pub const fn new() -> Self {
        Self {
            esp_enabled: true,
            esp_show_box: true,
            esp_show_names: false,
            esp_show_distance: false,
            esp_max_distance: 200.0,
            esp_min_distance: 2.0,
            esp_color_visible: [0.0, 1.0, 0.2, 1.0],
            esp_color_invisible: [1.0, 0.0, 0.0, 1.0],

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
            debug_camera_ok: false,
            debug_camera_loc: [0.0; 3],
            debug_camera_rot: [0.0; 3],
            debug_camera_fov: 0.0,
            debug_camera_source: 0,
            debug_pov_private: [0.0; 7],
            debug_pov_viewtarget: [0.0; 7],
            debug_pov_public: [0.0; 7],

            class_filter_active: true,
            class_groups: [crate::memory::ClassGroup {
                class_ptr: 0,
                count: 0,
            }; crate::memory::CLASS_GROUP_COUNT],
            selected_classes: [0; crate::memory::SELECTED_CLASS_COUNT],
            debug_player_class: 0,
            class_min_count: 1,
            class_max_count: 100,
            class_search: String::new(),

            esp_box_height_cm: 500.0,
        }
    }
}
