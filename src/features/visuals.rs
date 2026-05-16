use hudhook::imgui::*;
use std::sync::{Mutex, OnceLock};
use crate::state::ModState;
use crate::memory::{self, CameraChain};
use crate::features::filter::{self, ClassFilter};
use crate::features::raytrace;

struct EnemyFilter {
    glbase_class: usize,
    human_class: usize,
    human_player_class: usize,
    filter: ClassFilter,
    initialized: bool,
    last_stats: filter::GObjectsStats,
}

static ENEMY_FILTER: OnceLock<Mutex<EnemyFilter>> = OnceLock::new();

fn enemy_filter() -> &'static Mutex<EnemyFilter> {
    ENEMY_FILTER.get_or_init(|| {
        Mutex::new(EnemyFilter {
            glbase_class: 0,
            human_class: 0,
            human_player_class: 0,
            filter: ClassFilter::new(Vec::new()),
            initialized: false,
            last_stats: filter::GObjectsStats::default(),
        })
    })
}

fn ensure_enemy_filter(module_base: usize) {
    if module_base == 0 {
        return;
    }
    let mut f = match enemy_filter().lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if f.initialized {
        return;
    }

    // Single-pass GObjects walk — finds all three classes in one sweep
    // (much cheaper than three separate full walks).
    let targets = ["GLBaseCharacter", "HumanPlayer", "Human"];
    let (results, stats) =
        filter::find_classes_by_names(module_base, memory::GOBJECTS_OFFSET, &targets);
    f.last_stats = stats;

    let glbase = results[0].unwrap_or(0);
    let human_player = results[1].unwrap_or(0);
    let human = results[2].unwrap_or(0);

    // Order matters: HumanPlayer must be hit BEFORE Human while walking the
    // SuperStruct chain so the local player is distinguished from generic
    // human enemies.
    let roots = vec![glbase, human_player, human];
    f.glbase_class = glbase;
    f.human_class = human;
    f.human_player_class = human_player;
    f.filter = ClassFilter::new(roots);
    f.initialized = glbase != 0 && human != 0 && human_player != 0;
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum FilterMatch {
    #[default]
    None,
    Enemy(memory::EnemyKind),
    LocalPlayer,
}

fn classify_actor(f: &mut EnemyFilter, class_ptr: usize) -> FilterMatch {
    match f.filter.classify(class_ptr) {
        Some(root) if root == f.human_player_class => FilterMatch::LocalPlayer,
        Some(root) if root == f.glbase_class => FilterMatch::Enemy(memory::EnemyKind::Mech),
        Some(root) if root == f.human_class => FilterMatch::Enemy(memory::EnemyKind::Human),
        _ => FilterMatch::None,
    }
}

pub fn is_enemy_class(class_ptr: usize) -> bool {
    let mut f = match enemy_filter().lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    matches!(classify_actor(&mut f, class_ptr), FilterMatch::Enemy(_))
}

pub fn is_human_pawn_class(class_ptr: usize) -> bool {
    let mut f = match enemy_filter().lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    matches!(
        classify_actor(&mut f, class_ptr),
        FilterMatch::LocalPlayer | FilterMatch::Enemy(memory::EnemyKind::Human)
    )
}

pub fn render_main_tab(ui: &Ui, state: &mut ModState) {
    ui.text("ESP Settings");
    ui.separator();

    ui.checkbox("Enemy ESP", &mut state.esp_enabled);
    ui.checkbox("Show Box", &mut state.esp_show_box);
    ui.checkbox("Show Names", &mut state.esp_show_names);
    ui.checkbox("Show Distance", &mut state.esp_show_distance);

    ui.text("Min Distance (m):");
    ui.slider("##min_dist", 0.0, 50.0, &mut state.esp_min_distance);
    ui.text("Max Distance (m):");
    ui.slider("##max_dist", 10.0, 1000.0, &mut state.esp_max_distance);

    ui.separator();
    ui.text("Visible Color:");
    ui.color_edit4("##esp_color_vis", &mut state.esp_color_visible);
    ui.text("Invisible Color:");
    ui.color_edit4("##esp_color_invis", &mut state.esp_color_invisible);
}

fn build_axes(rotation: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let pitch = rotation[0].to_radians();
    let yaw = rotation[1].to_radians();
    let roll = rotation[2].to_radians();

    let cp = pitch.cos();
    let sp = pitch.sin();
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cr = roll.cos();
    let sr = roll.sin();

    let forward = [cp * cy, cp * sy, sp];
    let right = [
        sr * sp * cy - cr * sy,
        sr * sp * sy + cr * cy,
        -sr * cp,
    ];
    let up = [
        -(cr * sp * cy + sr * sy),
        -(cr * sp * sy - sr * cy),
        cr * cp,
    ];
    (forward, right, up)
}

struct ProjView {
    cam_loc: [f32; 3],
    forward: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    half_w: f32,
    half_h: f32,
    scale: f32,
    screen_w: f32,
    screen_h: f32,
}

fn make_proj_view(camera: &CameraChain, screen_size: [f32; 2]) -> Option<ProjView> {
    let (forward, right, up) = build_axes(camera.rotation);
    let half_w = screen_size[0] * 0.5;
    let half_h = screen_size[1] * 0.5;
    let fov_tan = (camera.fov.to_radians() * 0.5).tan();
    if !fov_tan.is_finite() || fov_tan.abs() < 1e-6 {
        return None;
    }
    let scale = half_w / fov_tan;
    Some(ProjView {
        cam_loc: camera.location,
        forward,
        right,
        up,
        half_w,
        half_h,
        scale,
        screen_w: screen_size[0],
        screen_h: screen_size[1],
    })
}

fn project(view: &ProjView, world_pos: [f32; 3]) -> Option<([f32; 2], f32)> {
    let dx = world_pos[0] - view.cam_loc[0];
    let dy = world_pos[1] - view.cam_loc[1];
    let dz = world_pos[2] - view.cam_loc[2];

    let local_x = dx * view.forward[0] + dy * view.forward[1] + dz * view.forward[2];
    if local_x < 1.0 {
        return None;
    }
    let local_y = dx * view.right[0] + dy * view.right[1] + dz * view.right[2];
    let local_z = dx * view.up[0] + dy * view.up[1] + dz * view.up[2];

    let sx = view.half_w + (local_y * view.scale / local_x);
    let sy = view.half_h - (local_z * view.scale / local_x);

    if !sx.is_finite() || !sy.is_finite() {
        return None;
    }
    let margin = 200.0;
    if sx < -margin || sx > view.screen_w + margin
        || sy < -margin || sy > view.screen_h + margin {
        return None;
    }
    Some(([sx, sy], local_x))
}

pub fn draw_esp(ui: &Ui, state: &mut ModState) {
    memory::clear_region_cache();

    let base = memory::get_module_base();
    state.debug_base_addr = base;
    ensure_enemy_filter(base);
    let line_trace_ready = raytrace::ensure_initialized(base);
    state.debug_line_trace_ok = line_trace_ready;
    state.debug_line_trace_fn = raytrace::line_trace_fn();
    if let Ok(f) = enemy_filter().lock() {
        state.debug_filter_glbase_class = f.glbase_class;
        state.debug_filter_human_class = f.human_class;
        state.debug_filter_human_player_class = f.human_player_class;
        state.debug_filter_init_ok = f.initialized;
        state.debug_filter_gobjects_addr = f.last_stats.gobjects_addr;
        state.debug_filter_chunks_array = f.last_stats.chunks_array;
        state.debug_filter_num_elements = f.last_stats.num_elements;
        state.debug_filter_num_elements_offset = f.last_stats.num_elements_offset;
        state.debug_filter_visited = f.last_stats.visited;
    }
    state.debug_filter_probe = filter::probe_gobjects_layout(base, memory::GOBJECTS_OFFSET);

    let world = memory::get_gworld(base);
    state.debug_world_addr = world;

    let (level, actors) = memory::get_actors(world);
    state.debug_level_addr = level;
    state.debug_actor_count = actors.count;
    state.debug_visible_actors = 0;

    let camera = memory::get_camera_chain(world);
    state.debug_gi = camera.gi;
    state.debug_lp_array = camera.lp_array;
    state.debug_local_player = camera.local_player;
    state.debug_pc = camera.pc;
    state.debug_cm = camera.cm;
    state.debug_camera_ok = camera.ok;
    state.debug_camera_loc = camera.location;
    state.debug_camera_rot = camera.rotation;
    state.debug_camera_fov = camera.fov;
    state.debug_camera_source = camera.source;

    let pack = |p: &memory::PovSample| -> [f32; 7] {
        [p.location[0], p.location[1], p.location[2],
         p.rotation[0], p.rotation[1], p.rotation[2],
         p.fov]
    };
    state.debug_pov_private = pack(&camera.pov_private);
    state.debug_pov_viewtarget = pack(&camera.pov_viewtarget);
    state.debug_pov_public = pack(&camera.pov_public);

    if !state.esp_enabled {
        return;
    }
    if !camera.ok {
        return;
    }
    if actors.count <= 0 || actors.data == 0 {
        return;
    }

    let [screen_w, screen_h] = ui.io().display_size;
    if !screen_w.is_finite() || !screen_h.is_finite() || screen_w < 1.0 || screen_h < 1.0 {
        return;
    }

    let view = match make_proj_view(&camera, [screen_w, screen_h]) {
        Some(v) => v,
        None => return,
    };

    let draw_list = ui.get_background_draw_list();
    let color_visible = state.esp_color_visible;
    let color_invisible = state.esp_color_invisible;
    let min_dist_cm = state.esp_min_distance * 100.0;
    let max_dist_cm = state.esp_max_distance * 100.0;
    let min_dist_sq = min_dist_cm * min_dist_cm;
    let max_dist_sq = max_dist_cm * max_dist_cm;
    let mut visible = 0i32;

    let player_pawn = memory::get_player_pawn(camera.pc);
    state.debug_player_class = memory::get_actor_class(player_pawn);

    let mut groups: Vec<memory::ClassGroup> = Vec::with_capacity(64);
    let manual_filter_on = state.class_filter_active
        && state.selected_classes.iter().any(|&c| c != 0);
    let show_names = state.esp_show_names;
    let show_distance = state.esp_show_distance;
    let show_labels = show_names || show_distance;
    let module_base = state.debug_base_addr;

    let mut name_cache: std::collections::HashMap<usize, String> =
        std::collections::HashMap::with_capacity(64);
    let mut filter_guard = enemy_filter().lock().expect("enemy filter poisoned");

    let actor_ptrs = memory::actor_slice(&actors);
    for &actor in actor_ptrs {
        if actor == 0 { continue; }
        // The Greylock player pawn is BP_Human_Player_C → ABP_GLHuman_C → AHuman
        // (no AHumanPlayer in the chain), so it would classify as Enemy(Human).
        // Skip by pawn pointer instead of relying on class.
        if player_pawn != 0 && actor == player_pawn { continue; }

        let class_ptr = memory::get_actor_class(actor);

        if class_ptr != 0 {
            if let Some(g) = groups.iter_mut().find(|g| g.class_ptr == class_ptr) {
                g.count += 1;
            } else {
                groups.push(memory::ClassGroup { class_ptr, count: 1 });
            }
        }

        // Filter chain: inheritance match → exclude HumanPlayer → alive → in range.
        let class_match = if class_ptr != 0 {
            classify_actor(&mut filter_guard, class_ptr)
        } else {
            FilterMatch::None
        };

        let manual_match = manual_filter_on
            && class_ptr != 0
            && state.selected_classes.iter().any(|&c| c == class_ptr);

        let kind = match class_match {
            FilterMatch::Enemy(k) => k,
            FilterMatch::LocalPlayer => {
                if !manual_match { continue; }
                memory::EnemyKind::None
            }
            FilterMatch::None => {
                if !manual_match { continue; }
                memory::EnemyKind::None
            }
        };

        if kind != memory::EnemyKind::None && !memory::is_actor_alive(actor, kind) {
            continue;
        }

        let loc = match memory::get_actor_location(actor) {
            Some(l) => l,
            None => continue,
        };

        let dx = loc[0] - view.cam_loc[0];
        let dy = loc[1] - view.cam_loc[1];
        let dz = loc[2] - view.cam_loc[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if !dist_sq.is_finite() { continue; }
        if dist_sq < min_dist_sq || dist_sq > max_dist_sq { continue; }

        let (screen, depth) = match project(&view, loc) {
            Some(s) => s,
            None => continue,
        };

        let dist = dist_sq.sqrt();
        visible += 1;

        let actor_visible = if kind != memory::EnemyKind::None && line_trace_ready {
            let ignore = [player_pawn, actor];
            raytrace::line_of_sight(world, view.cam_loc, loc, &ignore)
        } else {
            true
        };
        let color = if actor_visible { color_visible } else { color_invisible };

        if state.esp_show_box {
            // Auto-size from the actor's capsule (root component). Fall back to a
            // human-sized box when the read fails or the actor isn't a Character.
            let (height_cm, width_cm) = memory::get_actor_capsule(actor)
                .unwrap_or((180.0, 70.0));
            let pixels_per_cm = view.scale / depth;
            let box_h = (height_cm * pixels_per_cm).max(4.0);
            let box_w = (width_cm * pixels_per_cm).max(2.0);
            let half_w = box_w * 0.5;
            let half_h = box_h * 0.5;
            draw_list
                .add_rect(
                    [screen[0] - half_w, screen[1] - half_h],
                    [screen[0] + half_w, screen[1] + half_h],
                    color,
                )
                .thickness(1.5)
                .build();
        }

        if show_labels && class_ptr != 0 {
            let name = name_cache.entry(class_ptr).or_insert_with(|| {
                memory::get_class_name(module_base, class_ptr)
                    .unwrap_or_else(|| format!("0x{:X}", class_ptr))
            });
            let text = if show_names && show_distance {
                format!("{}\n{:.0}m", name, dist * 0.01)
            } else if show_names {
                name.clone()
            } else {
                format!("{:.0}m", dist * 0.01)
            };
            draw_list.add_text(
                [screen[0] - 40.0, screen[1] + 4.0],
                color,
                text,
            );
        }
    }
    drop(filter_guard);

    state.debug_visible_actors = visible;

    groups.sort_by(|a, b| b.count.cmp(&a.count));
    for slot in state.class_groups.iter_mut() {
        *slot = memory::ClassGroup::default();
    }
    for (i, g) in groups.iter().take(state.class_groups.len()).enumerate() {
        state.class_groups[i] = *g;
    }
}
