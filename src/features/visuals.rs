use hudhook::imgui::*;
use crate::state::ModState;
use crate::memory;

pub fn render_esp_ui(ui: &Ui, state: &mut ModState) {
    ui.text("ESP Settings");
    ui.separator();

    ui.checkbox("Enable ESP", &mut state.esp_enabled);
    ui.checkbox("Show Box", &mut state.esp_show_box);
    ui.checkbox("Show Health", &mut state.esp_show_health);
    ui.checkbox("Show Distance", &mut state.esp_show_distance);
    ui.checkbox("Show Names", &mut state.esp_show_names);

    ui.text("Max Distance:");
    ui.slider("##max_dist", 100.0, 5000.0, &mut state.esp_max_distance);

    ui.text("Color:");
    ui.color_edit4("##esp_color", &mut state.esp_color);

    ui.separator();
    ui.text("Debug Info:");
    ui.text(format!("Module Base: 0x{:X}", state.debug_base_addr));
    ui.text(format!("GWorld Addr: 0x{:X}", state.debug_world_addr));
    ui.text(format!("Level Addr:  0x{:X}", state.debug_level_addr));
    ui.text(format!("Actor Count: {}", state.debug_actor_count));

    ui.separator();
    ui.text("Offset Scan (level_off, actor_off, count):");
    ui.text("Look for a count between 50-5000:");
    for (lv, ar, count) in state.debug_scan.iter() {
        if *lv == 0 && *ar == 0 {
            continue;
        }
        let highlight = *count > 50 && *count < 5000;
        if highlight {
            ui.text_colored([0.0, 1.0, 0.0, 1.0],
                format!("  0x{:X} + 0x{:X} -> {}  <-- candidate", lv, ar, count));
        } else {
            ui.text(format!("  0x{:X} + 0x{:X} -> {}", lv, ar, count));
        }
    }
}

fn is_sane(v: f32) -> bool {
    v.is_finite() && v.abs() < 1.0e7
}

pub fn draw_esp(ui: &Ui, state: &mut ModState) {
    let base = memory::get_module_base();
    state.debug_base_addr = base;

    let world = memory::get_gworld(base);
    state.debug_world_addr = world;

    state.debug_scan = memory::scan_offsets(world);

    if !state.esp_enabled {
        return;
    }

    let (level, actors) = memory::find_best_actors(world);
    state.debug_level_addr = level;
    state.debug_actor_count = actors.count;

    if actors.count <= 0 || actors.data == 0 {
        return;
    }

    let draw_list = ui.get_background_draw_list();
    let [screen_w, screen_h] = ui.io().display_size;
    if !is_sane(screen_w) || !is_sane(screen_h) || screen_w < 1.0 || screen_h < 1.0 {
        return;
    }
    let color = state.esp_color;

    for i in 0..actors.count {
        let actor = memory::get_actor(&actors, i);
        if actor == 0 {
            continue;
        }

        let loc = match memory::get_actor_location(actor) {
            Some(l) => l,
            None => continue,
        };

        if !is_sane(loc[0]) || !is_sane(loc[1]) || !is_sane(loc[2]) {
            continue;
        }

        let screen_x = screen_w * 0.5 + (loc[0] * 0.01);
        let screen_y = screen_h * 0.5 - (loc[2] * 0.01);

        if !is_sane(screen_x) || !is_sane(screen_y) {
            continue;
        }

        if screen_x < 0.0 || screen_x > screen_w || screen_y < 0.0 || screen_y > screen_h {
            continue;
        }

        if state.esp_show_box {
            draw_list
                .add_rect(
                    [screen_x - 15.0, screen_y - 30.0],
                    [screen_x + 15.0, screen_y + 30.0],
                    color,
                )
                .thickness(1.5)
                .build();
        }

        if state.esp_show_distance {
            let dist_sq = loc[0] * loc[0] + loc[1] * loc[1] + loc[2] * loc[2];
            if !is_sane(dist_sq) {
                continue;
            }
            let dist = dist_sq.sqrt() * 0.01;
            if !is_sane(dist) {
                continue;
            }
            draw_list.add_text(
                [screen_x - 20.0, screen_y + 32.0],
                color,
                format!("{:.0}m", dist),
            );
        }
    }
}
