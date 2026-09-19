//! Initialisiert die Tauri-Anwendung, registriert Schnittstellen und verwaltet den Anwendungszustand.

mod application;
mod commands;
mod domain;
mod import_files;
mod importers;
mod infrastructure;
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
    mapping: Option<importers::TabularMapping>,
) -> Result<importers::ParsedStatement, String> {
    let _lease = storage.require_unlocked()?;
    importers::parse_statement(path, selected_provider, mapping)
}

#[tauri::command]
fn inspect_tabular_file(
    storage: tauri::State<'_, storage::Storage>,
    path: String,
) -> Result<importers::TabularInspection, String> {
    let _lease = storage.require_unlocked()?;
    importers::inspect_tabular_file(path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(application::assistant::account::ChatState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                window
                    .state::<application::assistant::account::ChatState>()
                    .stop();
            }
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
                handle
                    .state::<application::assistant::account::ChatState>()
                    .expire(&handle.state::<storage::Storage>());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::database::vault_status,
            commands::assistant::prepare_finance_chat,
            commands::assistant::chatgpt_status,
            commands::assistant::chatgpt_login_start,
            commands::assistant::chatgpt_disconnect,
            commands::assistant::cancel_finance_chat,
            commands::assistant::send_finance_chat,
            commands::database::create_demo_database,
            commands::database::demo_status,
            commands::database::create_backup,
            commands::database::restore_backup,
            commands::database::list_databases,
            commands::database::create_database,
            commands::database::copy_database,
            commands::database::switch_database,
            commands::database::anonymize_database,
            commands::database::anonymize_database_with_factor,
            commands::database::delete_database,
            commands::database::save_app_settings,
            commands::database::change_vault_password,
            commands::database::unlock_vault,
            commands::database::lock_vault,
            commands::database::vault_activity,
            parse_statement,
            inspect_tabular_file,
            import_files::collect_import_files,
            commands::imports::save_import,
            commands::imports::check_import_duplicates,
            commands::imports::is_file_imported,
            commands::imports::list_imports,
            commands::imports::list_import_mapping_profiles,
            commands::imports::save_import_mapping_profile,
            commands::imports::delete_imports,
            commands::reporting::database_status,
            commands::reporting::dashboard_data,
            commands::accounts::list_accounts,
            commands::reporting::wealth_data,
            commands::position_history::position_chart_data,
            commands::transactions::transaction_analysis,
            commands::transactions::set_transaction_settlement,
            commands::transactions::set_transaction_transfers,
            commands::cards::list_card_setup_transactions,
            commands::cards::set_card_credit_decision,
            commands::cards::confirm_card_setup,
            commands::rules::preview_settlement_rule,
            commands::rules::confirm_settlement_rule,
            commands::rules::list_settlement_rules,
            commands::rules::list_transfer_rules,
            commands::rules::preview_transfer_rule,
            commands::rules::save_transfer_rule,
            commands::rules::delete_settlement_rule,
            commands::transactions::list_transaction_transfers,
            commands::transactions::set_transaction_category,
            commands::categories::list_categories,
            commands::categories::save_category,
            commands::categories::remove_category,
            commands::categories::list_industry_rules,
            commands::categories::save_industry_rule,
            commands::accounts::create_account,
            commands::accounts::update_account,
            commands::accounts::delete_account,
            commands::positions::save_manual_valuation,
            commands::positions::list_manual_positions,
            commands::positions::delete_manual_position,
            commands::market_data::refresh_market_data,
            commands::accounts::set_institution_logo,
            commands::taxes::preview_tax_statement,
            commands::taxes::save_tax_statement,
            commands::taxes::save_manual_tax_snapshot,
            commands::taxes::list_tax_snapshots,
            commands::taxes::update_tax_snapshot,
            commands::taxes::delete_tax_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
