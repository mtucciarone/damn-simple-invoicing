mod commands;
mod db;
mod error;
mod models;
mod services;
mod state;

use tauri::Manager;

use crate::db::Database;
use crate::state::AppState;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let database = Database::from_app_handle(app.handle())?;
            database.migrate()?;
            app.manage(AppState::new(database));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::get_app_settings,
            commands::update_app_settings,
            commands::list_businesses,
            commands::get_active_business,
            commands::get_business,
            commands::create_business,
            commands::update_business,
            commands::archive_business,
            commands::set_active_business,
            commands::list_clients,
            commands::get_client,
            commands::create_client,
            commands::update_client,
            commands::archive_client,
            commands::delete_client,
            commands::create_invoice,
            commands::update_invoice,
            commands::duplicate_invoice,
            commands::delete_invoice,
            commands::delete_draft_invoice,
            commands::finalize_invoice,
            commands::reorder_invoice_line_items,
            commands::get_invoice,
            commands::list_invoices,
            commands::record_payment,
            commands::update_payment,
            commands::delete_payment,
            commands::get_payment,
            commands::list_payments,
            commands::get_dashboard_summary,
            commands::export_invoices_csv,
            commands::export_payments_csv,
            commands::export_database_backup,
            commands::restore_database_backup,
            commands::open_local_path,
            commands::export_invoice_pdf,
            commands::open_invoice_pdf,
            commands::export_invoice_html_only
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
