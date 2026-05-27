use std::collections::HashMap;
use std::path::PathBuf;

use tauri::State;

use crate::error::AppResult;
use crate::models::*;
use crate::services;
use crate::state::AppState;

fn to_command_result<T>(result: AppResult<T>) -> Result<T, String> {
    result.map_err(String::from)
}

#[tauri::command]
pub fn get_app_state(state: State<'_, AppState>) -> Result<AppBootState, String> {
    to_command_result(services::get_app_state(&state.database))
}

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
    let connection = state.database.open().map_err(String::from)?;
    to_command_result(services::load_settings_map(&connection))
}

#[tauri::command]
pub fn update_app_settings(
    state: State<'_, AppState>,
    settings: HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    to_command_result(services::update_settings(&state.database, settings))
}

#[tauri::command]
pub fn list_businesses(
    state: State<'_, AppState>,
    include_archived: bool,
) -> Result<Vec<BusinessProfile>, String> {
    to_command_result(services::list_businesses(&state.database, include_archived))
}

#[tauri::command]
pub fn get_active_business(state: State<'_, AppState>) -> Result<Option<BusinessProfile>, String> {
    let connection = state.database.open().map_err(String::from)?;
    to_command_result(services::get_active_business_record(&connection))
}

#[tauri::command]
pub fn get_business(state: State<'_, AppState>, id: i64) -> Result<BusinessProfile, String> {
    to_command_result(services::get_business(&state.database, id))
}

#[tauri::command]
pub fn create_business(
    state: State<'_, AppState>,
    input: BusinessProfileInput,
) -> Result<BusinessProfile, String> {
    to_command_result(services::create_business(&state.database, input))
}

#[tauri::command]
pub fn update_business(
    state: State<'_, AppState>,
    id: i64,
    input: BusinessProfileInput,
) -> Result<BusinessProfile, String> {
    to_command_result(services::update_business(&state.database, id, input))
}

#[tauri::command]
pub fn archive_business(state: State<'_, AppState>, id: i64) -> Result<BusinessProfile, String> {
    to_command_result(services::archive_business(&state.database, id))
}

#[tauri::command]
pub fn set_active_business(state: State<'_, AppState>, id: i64) -> Result<BusinessProfile, String> {
    to_command_result(services::set_active_business(&state.database, id))
}

#[tauri::command]
pub fn list_clients(
    state: State<'_, AppState>,
    search: Option<String>,
    include_archived: bool,
) -> Result<Vec<Client>, String> {
    to_command_result(services::list_clients(
        &state.database,
        search,
        include_archived,
    ))
}

#[tauri::command]
pub fn get_client(state: State<'_, AppState>, id: i64) -> Result<ClientDetail, String> {
    to_command_result(services::get_client(&state.database, id))
}

#[tauri::command]
pub fn create_client(state: State<'_, AppState>, input: ClientInput) -> Result<Client, String> {
    to_command_result(services::create_client(&state.database, input))
}

#[tauri::command]
pub fn update_client(
    state: State<'_, AppState>,
    id: i64,
    input: ClientInput,
) -> Result<Client, String> {
    to_command_result(services::update_client(&state.database, id, input))
}

#[tauri::command]
pub fn archive_client(state: State<'_, AppState>, id: i64) -> Result<Client, String> {
    to_command_result(services::archive_client(&state.database, id))
}

#[tauri::command]
pub fn delete_client(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    to_command_result(services::delete_client(&state.database, id))
}

#[tauri::command]
pub fn create_invoice(
    state: State<'_, AppState>,
    input: InvoiceFormInput,
) -> Result<InvoiceDetail, String> {
    to_command_result(services::create_invoice(&state.database, input))
}

#[tauri::command]
pub fn update_invoice(
    state: State<'_, AppState>,
    id: i64,
    input: InvoiceFormInput,
) -> Result<InvoiceDetail, String> {
    to_command_result(services::update_invoice(&state.database, id, input))
}

#[tauri::command]
pub fn duplicate_invoice(state: State<'_, AppState>, id: i64) -> Result<InvoiceDetail, String> {
    to_command_result(services::duplicate_invoice(&state.database, id))
}

#[tauri::command]
pub fn delete_invoice(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    to_command_result(services::delete_invoice(&state.database, id))
}

#[tauri::command]
pub fn delete_draft_invoice(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    to_command_result(services::delete_draft_invoice(&state.database, id))
}

#[tauri::command]
pub fn finalize_invoice(state: State<'_, AppState>, id: i64) -> Result<InvoiceDetail, String> {
    to_command_result(services::finalize_invoice(&state.database, id))
}

#[tauri::command]
pub fn reorder_invoice_line_items(
    state: State<'_, AppState>,
    invoice_id: i64,
    ordered_line_item_ids: Vec<i64>,
) -> Result<(), String> {
    to_command_result(services::reorder_invoice_line_items(
        &state.database,
        invoice_id,
        ordered_line_item_ids,
    ))
}

#[tauri::command]
pub fn get_invoice(state: State<'_, AppState>, invoice_id: i64) -> Result<InvoiceDetail, String> {
    to_command_result(services::get_invoice(&state.database, invoice_id))
}

#[tauri::command]
pub fn list_invoices(
    state: State<'_, AppState>,
    filters: InvoiceFilters,
) -> Result<Vec<InvoiceSummary>, String> {
    to_command_result(services::list_invoices(&state.database, filters))
}

#[tauri::command]
pub fn record_payment(state: State<'_, AppState>, input: PaymentInput) -> Result<Payment, String> {
    to_command_result(services::record_payment(&state.database, input))
}

#[tauri::command]
pub fn update_payment(
    state: State<'_, AppState>,
    payment_id: i64,
    input: PaymentInput,
) -> Result<Payment, String> {
    to_command_result(services::update_payment(&state.database, payment_id, input))
}

#[tauri::command]
pub fn delete_payment(state: State<'_, AppState>, payment_id: i64) -> Result<(), String> {
    to_command_result(services::delete_payment(&state.database, payment_id))
}

#[tauri::command]
pub fn get_payment(state: State<'_, AppState>, payment_id: i64) -> Result<Payment, String> {
    to_command_result(services::get_payment(&state.database, payment_id))
}

#[tauri::command]
pub fn list_payments(
    state: State<'_, AppState>,
    filters: PaymentFilters,
) -> Result<Vec<Payment>, String> {
    to_command_result(services::list_payments(&state.database, filters))
}

#[tauri::command]
pub fn get_dashboard_summary(
    state: State<'_, AppState>,
    filters: DashboardFilters,
) -> Result<DashboardSummary, String> {
    to_command_result(services::get_dashboard_summary(&state.database, filters))
}

#[tauri::command]
pub fn export_invoices_csv(
    state: State<'_, AppState>,
    path: String,
    filters: Option<DashboardFilters>,
) -> Result<CsvExportResult, String> {
    let output_path = PathBuf::from(path);
    to_command_result(services::export_invoices_csv(
        &state.database,
        &output_path,
        filters,
    ))?;
    Ok(CsvExportResult {
        path: output_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn export_payments_csv(
    state: State<'_, AppState>,
    path: String,
    filters: Option<DashboardFilters>,
) -> Result<CsvExportResult, String> {
    let output_path = PathBuf::from(path);
    to_command_result(services::export_payments_csv(
        &state.database,
        &output_path,
        filters,
    ))?;
    Ok(CsvExportResult {
        path: output_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn export_database_backup(
    state: State<'_, AppState>,
    destination_dir: String,
    zip: bool,
) -> Result<BackupExportResult, String> {
    let output_path = to_command_result(services::export_database_backup(
        &state.database,
        &PathBuf::from(destination_dir),
        zip,
    ))?;
    Ok(BackupExportResult {
        path: output_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn restore_database_backup(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<(), String> {
    to_command_result(services::restore_database_backup(
        &state.database,
        &PathBuf::from(source_path),
    ))
}

#[tauri::command]
pub fn open_local_path(path: String) -> Result<(), String> {
    to_command_result(services::open_local_path(&PathBuf::from(path)))
}

#[tauri::command]
pub fn generate_invoice_preview(
    state: State<'_, AppState>,
    invoice_id: i64,
    output_dir: Option<String>
) -> Result<PdfExportResult, String> {
    to_command_result(services::generate_invoice_preview(
        &state.database,
        invoice_id,
        output_dir.map(PathBuf::from)
    ))
}

#[tauri::command]
pub fn open_invoice_pdf(
    state: State<'_, AppState>,
    invoice_id: i64,
    output_dir: Option<String>,
) -> Result<PdfExportResult, String> {
    to_command_result(services::open_invoice_pdf(
        &state.database,
        invoice_id,
        output_dir.map(PathBuf::from),
    ))
}

#[tauri::command]
pub fn export_invoice_html_only(
    state: State<'_, AppState>,
    invoice_id: i64,
    output_dir: Option<String>,
) -> Result<PdfExportResult, String> {
    to_command_result(services::export_invoice_html_only(
        &state.database,
        invoice_id,
        output_dir.map(PathBuf::from),
    ))
}
