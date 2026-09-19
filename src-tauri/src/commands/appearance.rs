//! Verbindet die gerätebezogene Darstellung mit den plattformneutralen Tauri-Fenster-APIs.
use crate::infrastructure::appearance::{self, Appearance};
use tauri::Manager;

fn path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("appearance.json"))
}

#[tauri::command]
pub fn load_appearance(app: tauri::AppHandle) -> Appearance {
    path(&app).map(|p| appearance::load(&p)).unwrap_or_default()
}

#[tauri::command]
pub fn save_appearance(app: tauri::AppHandle, appearance: Appearance) -> Result<(), String> {
    appearance::save(&path(&app)?, appearance)
}

#[tauri::command]
pub fn show_themed_window(
    window: tauri::WebviewWindow,
    dark: bool,
    reveal: bool,
) -> Result<(), String> {
    let theme_result = window
        .set_theme(Some(if dark {
            tauri::Theme::Dark
        } else {
            tauri::Theme::Light
        }))
        .map_err(|e| e.to_string());
    // A platform refusing native title-bar styling must not leave the app hidden.
    if reveal {
        window.show().map_err(|e| e.to_string())?;
    }
    theme_result
}
