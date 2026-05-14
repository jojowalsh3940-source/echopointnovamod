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

pub fn render_esp_ui(ui: &Ui, state: &mut ModState) {
    if let Some(_tabs) = ui.tab_bar("##main_tabs") {
        if let Some(_t) = ui.tab_item("Main") {
            render_main_tab(ui, state);
        }
        if let Some(_t) = ui.tab_item("Debug") {
            render_debug_tab(ui, state);
        }
    }
}

fn render_main_tab(ui: &Ui, state: &mut ModState) {
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

fn render_debug_tab(ui: &Ui, state: &mut ModState) {
    let red = [1.0, 0.4, 0.4, 1.0];
    let grn = [0.4, 1.0, 0.4, 1.0];

    ui.text("World:");
    ui.text(format!("Module:   0x{:X}", state.debug_base_addr));
    ui.text(format!("GWorld:   0x{:X}", state.debug_world_addr));
    ui.text(format!("Level:    0x{:X}", state.debug_level_addr));
    ui.text(format!("Actors:   {}", state.debug_actor_count));
    ui.text(format!("Visible:  {}", state.debug_visible_actors));

    ui.separator();
    ui.text("Camera Chain:");
    let line = |ui: &Ui, label: &str, val: usize| {
        let c = if val == 0 { red } else { grn };
        ui.text_colored(c, format!("{} 0x{:X}", label, val));
    };
    line(ui, "GameInstance:", state.debug_gi);
    line(ui, "LPArray data:", state.debug_lp_array);
    line(ui, "LocalPlayer: ", state.debug_local_player);
    line(ui, "PlayerCtrl:  ", state.debug_pc);
    line(ui, "CameraMgr:   ", state.debug_cm);

    ui.separator();
    if state.debug_camera_ok {
        let src = match state.debug_camera_source {
            1 => "CameraCachePrivate",
            2 => "ViewTarget.POV",
            3 => "CameraCache (public)",
            _ => "unknown",
        };
        ui.text_colored(grn, format!("Camera: OK  [{}]", src));
    } else {
        ui.text_colored(red, "Camera: NOT FOUND");
    }
    ui.text(format!("Loc: {:.0} {:.0} {:.0}",
        state.debug_camera_loc[0], state.debug_camera_loc[1], state.debug_camera_loc[2]));
    ui.text(format!("Rot: {:.1} {:.1} {:.1}",
        state.debug_camera_rot[0], state.debug_camera_rot[1], state.debug_camera_rot[2]));
    ui.text(format!("FOV: {:.1}", state.debug_camera_fov));

    let pov_line = |ui: &Ui, label: &str, v: &[f32; 7]| {
        let valid = v[6] > 0.5;
        let color = if valid { grn } else { red };
        ui.text_colored(color, format!(
            "{}  loc={:.0},{:.0},{:.0}  rot={:.1},{:.1},{:.1}  fov={:.1}",
            label, v[0], v[1], v[2], v[3], v[4], v[5], v[6]
        ));
    };
    ui.separator();
    ui.text("POV Sources (compare which is live):");
    pov_line(ui, "Priv:", &state.debug_pov_private);
    pov_line(ui, "VT:  ", &state.debug_pov_viewtarget);
    pov_line(ui, "Pub: ", &state.debug_pov_public);

    ui.separator();
    ui.text("Class Filter:");
    ui.checkbox("Auto enemy filter (BP_Human_Enemy / BP_Harrier / BP_RoverBase)",
        &mut state.auto_enemy_filter);
    ui.checkbox("Manual class filter active", &mut state.class_filter_active);
    ui.text(format!("Player Pawn Class: 0x{:X}", state.debug_player_class));
    if ui.button("Toggle Player Class##togpc") {
        let pc = state.debug_player_class;
        if pc != 0 {
            let already = state.selected_classes.iter().any(|&c| c == pc);
            if already {
                for c in state.selected_classes.iter_mut() {
                    if *c == pc { *c = 0; }
                }
            } else {
                for c in state.selected_classes.iter_mut() {
                    if *c == 0 { *c = pc; break; }
                }
            }
        }
    }
    ui.same_line();
    if ui.button("Clear All##clrcls") {
        state.selected_classes = [0; crate::memory::SELECTED_CLASS_COUNT];
    }

    ui.text("Count range (hide bulky props like clouds / voxels):");
    ui.slider("##cls_min", 1, 500, &mut state.class_min_count);
    ui.same_line();
    ui.text("min");
    ui.slider("##cls_max", 1, 2000, &mut state.class_max_count);
    ui.same_line();
    ui.text("max");

    let mut visible_count = 0i32;
    for g in state.class_groups.iter() {
        if g.class_ptr != 0
            && g.count >= state.class_min_count
            && g.count <= state.class_max_count
        {
            visible_count += 1;
        }
    }
    ui.text(format!(
        "Top Classes (showing {} of up to {} — click to toggle):",
        visible_count, state.class_groups.len()
    ));

    let module_base = state.debug_base_addr;
    ui.child_window("##classlist")
        .size([0.0, 240.0])
        .build(|| {
            for i in 0..state.class_groups.len() {
                let g = state.class_groups[i];
                if g.class_ptr == 0 { continue; }
                if g.count < state.class_min_count || g.count > state.class_max_count {
                    continue;
                }
                let selected = state.selected_classes.iter().any(|&c| c == g.class_ptr);
                let mark = if selected { "[X]" } else { "[ ]" };
                let player_mark = if g.class_ptr == state.debug_player_class { " (player)" } else { "" };
                let name = memory::get_class_name(module_base, g.class_ptr)
                    .unwrap_or_else(|| format!("0x{:X}", g.class_ptr));
                let enemy_mark = if memory::is_enemy_class_name(&name) { " [enemy]" } else { "" };
                let label = format!(
                    "{} {}  (0x{:X})  n={}{}{}##cls{}",
                    mark, name, g.class_ptr, g.count, player_mark, enemy_mark, i
                );
                let style = if selected {
                    Some(ui.push_style_color(StyleColor::Button, [0.15, 0.55, 0.15, 1.0]))
                } else {
                    None
                };
                if ui.button(label) {
                    if selected {
                        for c in state.selected_classes.iter_mut() {
                            if *c == g.class_ptr { *c = 0; }
                        }
                    } else {
                        for c in state.selected_classes.iter_mut() {
                            if *c == 0 { *c = g.class_ptr; break; }
                        }
                    }
                }
                drop(style);
            }
        });
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
    let auto_filter_on = state.auto_enemy_filter;
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

        if auto_filter_on || manual_filter_on {
            let auto_match = auto_filter_on && kind != memory::EnemyKind::None;
            let manual_match = manual_filter_on
                && state.selected_classes.iter().any(|&c| c == class_ptr);
            if !auto_match && !manual_match {
                continue;
            }
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
