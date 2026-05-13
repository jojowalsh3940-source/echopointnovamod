use hudhook::imgui::*;
use crate::state::ModState;
use crate::memory::{self, CameraChain};

pub fn render_esp_ui(ui: &Ui, state: &mut ModState) {
    ui.text("ESP Settings");
    ui.separator();

    ui.checkbox("Enable ESP", &mut state.esp_enabled);
    ui.checkbox("Show Box", &mut state.esp_show_box);
    ui.checkbox("Show Distance", &mut state.esp_show_distance);

    ui.text("Min Distance (m):");
    ui.slider("##min_dist", 0.0, 50.0, &mut state.esp_min_distance);
    ui.text("Max Distance (m):");
    ui.slider("##max_dist", 10.0, 1000.0, &mut state.esp_max_distance);

    ui.text("Color:");
    ui.color_edit4("##esp_color", &mut state.esp_color);

    ui.separator();
    ui.text("World:");
    ui.text(format!("Module:   0x{:X}", state.debug_base_addr));
    ui.text(format!("GWorld:   0x{:X}", state.debug_world_addr));
    ui.text(format!("Level:    0x{:X}", state.debug_level_addr));
    ui.text(format!("Actors:   {}", state.debug_actor_count));
    ui.text(format!("Visible:  {}", state.debug_visible_actors));

    ui.separator();
    ui.text("Camera Chain:");
    let red = [1.0, 0.4, 0.4, 1.0];
    let grn = [0.4, 1.0, 0.4, 1.0];
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
    ui.checkbox("Class Filter Active", &mut state.class_filter_active);
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
                let label = format!(
                    "{} 0x{:X}  n={}  loc={:.0},{:.0},{:.0}{}##cls{}",
                    mark, g.class_ptr, g.count,
                    g.sample_loc[0], g.sample_loc[1], g.sample_loc[2],
                    player_mark, i
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

fn world_to_screen(world_pos: [f32; 3], camera: &CameraChain, screen_size: [f32; 2]) -> Option<[f32; 2]> {
    let (forward, right, up) = build_axes(camera.rotation);

    let dx = world_pos[0] - camera.location[0];
    let dy = world_pos[1] - camera.location[1];
    let dz = world_pos[2] - camera.location[2];

    let local_x = dx * forward[0] + dy * forward[1] + dz * forward[2];
    let local_y = dx * right[0] + dy * right[1] + dz * right[2];
    let local_z = dx * up[0] + dy * up[1] + dz * up[2];

    if local_x < 1.0 {
        return None;
    }

    let half_w = screen_size[0] * 0.5;
    let half_h = screen_size[1] * 0.5;
    let fov_tan = (camera.fov.to_radians() * 0.5).tan();
    if !fov_tan.is_finite() || fov_tan.abs() < 1e-6 {
        return None;
    }
    let scale = half_w / fov_tan;

    let sx = half_w + (local_y * scale / local_x);
    let sy = half_h - (local_z * scale / local_x);

    if !sx.is_finite() || !sy.is_finite() {
        return None;
    }
    Some([sx, sy])
}

pub fn draw_esp(ui: &Ui, state: &mut ModState) {
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

    let draw_list = ui.get_background_draw_list();
    let color = state.esp_color;
    let min_dist_cm = state.esp_min_distance * 100.0;
    let max_dist_cm = state.esp_max_distance * 100.0;
    let mut visible = 0i32;

    state.debug_player_class = memory::get_player_pawn_class(camera.pc);

    let mut groups: Vec<memory::ClassGroup> = Vec::with_capacity(64);
    let filter_on = state.class_filter_active
        && state.selected_classes.iter().any(|&c| c != 0);

    for i in 0..actors.count {
        let actor = memory::get_actor(&actors, i);
        if actor == 0 { continue; }

        let loc = match memory::get_actor_location(actor) {
            Some(l) => l,
            None => continue,
        };

        let class_ptr = memory::get_actor_class(actor);
        if class_ptr != 0 {
            if let Some(g) = groups.iter_mut().find(|g| g.class_ptr == class_ptr) {
                g.count += 1;
            } else {
                groups.push(memory::ClassGroup {
                    class_ptr,
                    count: 1,
                    sample_loc: loc,
                });
            }
        }

        if filter_on && !state.selected_classes.iter().any(|&c| c == class_ptr) {
            continue;
        }

        let dx = loc[0] - camera.location[0];
        let dy = loc[1] - camera.location[1];
        let dz = loc[2] - camera.location[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if !dist_sq.is_finite() { continue; }
        let dist = dist_sq.sqrt();
        if dist < min_dist_cm || dist > max_dist_cm { continue; }

        let screen = match world_to_screen(loc, &camera, [screen_w, screen_h]) {
            Some(s) => s,
            None => continue,
        };

        if screen[0] < 0.0 || screen[0] > screen_w || screen[1] < 0.0 || screen[1] > screen_h {
            continue;
        }

        visible += 1;

        if state.esp_show_box {
            let height = (1500.0 / dist).clamp(4.0, 200.0);
            let width = height * 0.5;
            draw_list
                .add_rect(
                    [screen[0] - width, screen[1] - height],
                    [screen[0] + width, screen[1] + height],
                    color,
                )
                .thickness(1.5)
                .build();
        }

        if state.esp_show_distance {
            let dist_m = dist * 0.01;
            draw_list.add_text(
                [screen[0] - 20.0, screen[1] + 4.0],
                color,
                format!("{:.0}m", dist_m),
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
