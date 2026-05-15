use hudhook::imgui::*;
use crate::state::ModState;
use crate::features;

pub fn render_ui(ui: &Ui, state: &mut ModState) {
    let [width, height] = ui.io().display_size;

    ui.window("Echo Point Nova Mod")
        .size([400.0, 500.0], Condition::FirstUseEver)
        .position([width / 2.0 - 200.0, height / 2.0 - 250.0], Condition::FirstUseEver)
        .build(|| {
            if let Some(_tabs) = ui.tab_bar("##main_tabs") {
                if let Some(_t) = ui.tab_item("Main") {
                    features::visuals::render_main_tab(ui, state);
                }
                if let Some(_t) = ui.tab_item("Debug") {
                    features::debug::render_debug_tab(ui, state);
                }
                if let Some(_t) = ui.tab_item("Misc") {
                    features::misc::render_misc_tab(ui, state);
                }
            }
        });

    features::visuals::draw_esp(ui, state);
    features::misc::tick(state);
}
