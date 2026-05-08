// Phase: 1
// Tauri app entry point

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    modutone_app::run();
}
