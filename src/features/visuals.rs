use hudhook::imgui::*;
use crate::state::ModState;
use crate::memory::{self, CameraChain};

struct ClassMeta {
    kind: memory::EnemyKind,
    name: String,
}

fn build_class_meta(module_base: usize, class_ptr: usize) -> ClassMeta {
    let name = memory::get_class_name(module_base, class_ptr)
        .unwrap_or_else(|| format!("0x{:X}", class_ptr));
    let kind = memory::classify_enemy(&name);
    ClassMeta { kind, name }
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

    ui.text("Box Height (cm):");
    ui.slider("##box_h", 60.0, 800.0, &mut state.esp_box_height_cm);

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
    memory::step_vis_cache();

    let base = memory::get_module_base();
    state.debug_base_addr = base;

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

    state.debug_player_class = memory::get_player_pawn_class(camera.pc);

    let mut groups: Vec<memory::ClassGroup> = Vec::with_capacity(64);
    let manual_filter_on = state.class_filter_active
        && state.selected_classes.iter().any(|&c| c != 0);
    let show_names = state.esp_show_names;
    let show_distance = state.esp_show_distance;
    let show_labels = show_names || show_distance;
    let module_base = state.debug_base_addr;

    let mut class_cache: std::collections::HashMap<usize, ClassMeta> =
        std::collections::HashMap::with_capacity(128);

    let actor_ptrs = memory::actor_slice(&actors);
    for &actor in actor_ptrs {
        if actor == 0 { continue; }

        let class_ptr = memory::get_actor_class(actor);

        let kind = if class_ptr != 0 {
            class_cache
                .entry(class_ptr)
                .or_insert_with(|| build_class_meta(module_base, class_ptr))
                .kind
        } else {
            memory::EnemyKind::None
        };

        if kind != memory::EnemyKind::None && !memory::is_actor_alive(actor, kind) {
            continue;
        }

        if class_ptr != 0 {
            if let Some(g) = groups.iter_mut().find(|g| g.class_ptr == class_ptr) {
                g.count += 1;
            } else {
                groups.push(memory::ClassGroup { class_ptr, count: 1 });
            }
        }

        let is_enemy = kind != memory::EnemyKind::None;
        let manual_match = manual_filter_on
            && state.selected_classes.iter().any(|&c| c == class_ptr);
        if !is_enemy && !manual_match {
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

        let actor_visible = if kind != memory::EnemyKind::None {
            memory::is_actor_visible(actor)
        } else {
            true
        };
        let color = if actor_visible { color_visible } else { color_invisible };

        if state.esp_show_box {
            let pixels_per_cm = view.scale / depth;
            let box_h = (state.esp_box_height_cm * pixels_per_cm).max(4.0);
            let aspect = match kind {
                memory::EnemyKind::Mech => 1.0,
                _ => 0.4,
            };
            let box_w = (box_h * aspect).max(2.0);
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
            let meta = class_cache
                .entry(class_ptr)
                .or_insert_with(|| build_class_meta(module_base, class_ptr));
            let text = if show_names && show_distance {
                format!("{}\n{:.0}m", meta.name, dist * 0.01)
            } else if show_names {
                meta.name.clone()
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

    state.debug_visible_actors = visible;

    groups.sort_by(|a, b| b.count.cmp(&a.count));
    for slot in state.class_groups.iter_mut() {
        *slot = memory::ClassGroup::default();
    }
    for (i, g) in groups.iter().take(state.class_groups.len()).enumerate() {
        state.class_groups[i] = *g;
    }
}
