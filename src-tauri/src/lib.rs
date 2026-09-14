mod domain;
mod import_files;
mod importers;
mod storage;

use tauri::{Emitter, Manager};

fn lock_if_minimized(handle: &tauri::AppHandle, minimized: bool) {
    if minimized {
        if let Some(storage) = handle.try_state::<storage::Storage>() {
            if storage.lock_session().unwrap_or(false) {
                let _ = handle.emit("vault-locked", ());
            }
        }
    }
}

#[tauri::command]
fn parse_statement(
    storage: tauri::State<'_, storage::Storage>,
    path: String,
    selected_provider: Option<String>,
) -> Result<importers::ParsedStatement, String> {
    let _lease = storage.require_unlocked()?;
    importers::parse_statement(path, selected_provider)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            // Native minimize changes do not reliably produce WebView visibility events.
            // Check the actual state: ordinary focus loss must not lock the vault.
            if matches!(
                event,
                tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Focused(_)
            ) {
                lock_if_minimized(window.app_handle(), window.is_minimized().unwrap_or(false));
            }
        })
        .setup(|app| {
            let storage = storage::Storage::initialize(app.handle())?;
            app.manage(storage);
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                for window in handle.webview_windows().values() {
                    lock_if_minimized(&handle, window.is_minimized().unwrap_or(false));
                }
                handle.state::<storage::Storage>().expire_session();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            storage::security::vault_status,
            storage::security::databases::list_databases,
            storage::security::databases::create_database,
            storage::security::databases::copy_database,
            storage::security::databases::switch_database,
            storage::security::databases::anonymize_database,
            storage::security::databases::anonymize_database_with_factor,
            storage::security::databases::delete_database,
            storage::security::save_app_settings,
            storage::security::change_vault_password,
            storage::security::unlock_vault,
            storage::security::lock_vault,
            storage::security::vault_activity,
            parse_statement,
            import_files::collect_import_files,
            storage::save_import,
            storage::check_import_duplicates,
            storage::is_file_imported,
            storage::list_imports,
            storage::delete_imports,
            storage::database_status,
            storage::dashboard_data,
            storage::list_accounts,
            storage::wealth_data,
            storage::transaction_analysis,
            storage::reconciliation::card_reconciliation,
            storage::set_transaction_category,
            storage::categories::list_categories,
            storage::categories::save_category,
            storage::categories::remove_category,
            storage::categories::list_industry_rules,
            storage::categories::save_industry_rule,
            storage::create_account,
            storage::update_account,
            storage::delete_account,
            storage::save_manual_valuation,
            storage::list_manual_positions,
            storage::delete_manual_position,
            storage::market_data::refresh_market_data,
            storage::set_institution_logo,
            storage::tax_history::preview_tax_statement,
            storage::tax_history::save_tax_statement,
            storage::tax_history::save_manual_tax_snapshot,
            storage::tax_history::list_tax_snapshots,
            storage::tax_history::update_tax_snapshot,
            storage::tax_history::delete_tax_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
