mod state;
mod memory;
mod gui;
mod features;

use hudhook::*;
use hudhook::hooks::dx12::ImguiDx12Hooks;
use std::sync::Mutex;

static MOD_STATE: Mutex<state::ModState> = Mutex::new(state::ModState::new());

pub struct ModRenderLoop;

impl ImguiRenderLoop for ModRenderLoop {
    fn render(&mut self, ui: &mut imgui::Ui) {
        let mut state = MOD_STATE.lock().unwrap();
        gui::render_ui(ui, &mut state);
    }
}

hudhook!(ImguiDx12Hooks, ModRenderLoop);
