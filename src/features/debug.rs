use hudhook::imgui::*;
use crate::state::ModState;
use crate::memory;
use crate::features::visuals;

pub fn render_debug_tab(ui: &Ui, state: &mut ModState) {
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
    ui.text("Enemy Filter (inheritance):");
    let init_color = if state.debug_filter_init_ok { grn } else { red };
    ui.text_colored(init_color, format!(
        "Init: {}", if state.debug_filter_init_ok { "OK" } else { "PENDING/FAILED" }
    ));
    line(ui, "GLBaseCharacter:", state.debug_filter_glbase_class);
    line(ui, "Human:          ", state.debug_filter_human_class);
    line(ui, "HumanPlayer:    ", state.debug_filter_human_player_class);
    line(ui, "GObjects:       ", state.debug_filter_gobjects_addr);
    let chunks_color = if state.debug_filter_chunks_array == 0 { red } else { grn };
    ui.text_colored(chunks_color,
        format!("ObjObjects: 0x{:X}", state.debug_filter_chunks_array));
    let n_color = if state.debug_filter_num_elements <= 0
        || state.debug_filter_num_elements > 4_000_000 { red } else { grn };
    ui.text_colored(n_color, format!(
        "NumElements: {}  (auto-detected @ +0x{:X})",
        state.debug_filter_num_elements,
        state.debug_filter_num_elements_offset
    ));
    ui.text(format!("Visited:     {}", state.debug_filter_visited));
    ui.text("Raw i32 probe (GObjects + 0x00..+0x1C):");
    for (i, v) in state.debug_filter_probe.iter().enumerate() {
        let off = i * 4;
        let color = if *v >= 1_000 && *v <= 4_000_000 { grn } else { [0.7, 0.7, 0.7, 1.0] };
        ui.text_colored(color, format!("  +0x{:02X}: {}", off, v));
    }

    ui.separator();
    ui.text("Line Trace (visibility):");
    let lt_color = if state.debug_line_trace_ok { grn } else { red };
    ui.text_colored(lt_color, format!(
        "Init: {}", if state.debug_line_trace_ok { "OK" } else { "PENDING/FAILED" }
    ));
    line(ui, "LineTraceSingle:", state.debug_line_trace_fn);

    ui.separator();
    ui.text("Class Filter:");
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

    let module_base = state.debug_base_addr;

    ui.text("Search:");
    ui.same_line();
    ui.set_next_item_width(-60.0);
    ui.input_text("##cls_search", &mut state.class_search).build();
    ui.same_line();
    if ui.button("Clear##clrsearch") {
        state.class_search.clear();
    }
    let search_lc = state.class_search.to_lowercase();
    let search_active = !search_lc.is_empty();

    let mut names: [Option<String>; crate::memory::CLASS_GROUP_COUNT] =
        std::array::from_fn(|_| None);
    let mut visible_count = 0i32;
    for (i, g) in state.class_groups.iter().enumerate() {
        if g.class_ptr == 0 { continue; }
        if g.count < state.class_min_count || g.count > state.class_max_count {
            continue;
        }
        let name = memory::get_class_name(module_base, g.class_ptr)
            .unwrap_or_else(|| format!("0x{:X}", g.class_ptr));
        if search_active && !name.to_lowercase().contains(&search_lc) {
            continue;
        }
        names[i] = Some(name);
        visible_count += 1;
    }

    ui.text(format!(
        "Top Classes (showing {} of up to {} — click to toggle):",
        visible_count, state.class_groups.len()
    ));

    ui.child_window("##classlist")
        .size([0.0, 240.0])
        .build(|| {
            for i in 0..state.class_groups.len() {
                let name = match names[i].take() {
                    Some(n) => n,
                    None => continue,
                };
                let g = state.class_groups[i];
                let selected = state.selected_classes.iter().any(|&c| c == g.class_ptr);
                let mark = if selected { "[X]" } else { "[ ]" };
                let player_mark = if g.class_ptr == state.debug_player_class { " (player)" } else { "" };
                let enemy_mark = if visuals::is_enemy_class(g.class_ptr) { " [enemy]" } else { "" };
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
