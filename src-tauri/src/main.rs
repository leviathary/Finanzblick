//! Startet die Tauri-Desktopanwendung und unterdrückt im Windows-Release das Konsolenfenster.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    finanzblick_lib::run()
}
