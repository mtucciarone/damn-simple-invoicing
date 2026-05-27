use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::{engine::general_purpose, Engine as _};
use chrono::{SecondsFormat, Utc};
use rusqlite::{named_params, params, types::Type, OptionalExtension, Row};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::{Decimal, RoundingStrategy};
use serde_json::json;
use zip::write::FileOptions;
use zip::ZipWriter;

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::*;

fn utc_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn today_date() -> String {
    Utc::now().date_naive().format("%Y-%m-%d").to_string()
}

fn validate_required(label: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        Err(AppError::Validation(format!("{label} is required")))
    } else {
        Ok(())
    }
}

fn parse_decimal(input: &str, label: &str) -> AppResult<Decimal> {
    Decimal::from_str_exact(input.trim())
        .map_err(|_| AppError::Validation(format!("{label} must be a valid decimal number")))
}

fn money_display(amount_minor: i64) -> String {
    let major = Decimal::from_i64(amount_minor).unwrap_or(Decimal::ZERO)
        / Decimal::from_i64(100).unwrap_or(Decimal::ONE);
    format!("{major:.2}")
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn find_executable(candidates: &[&str]) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let paths = std::env::split_paths(&path_var);

    for directory in paths {
        for candidate in candidates {
            #[cfg(windows)]
            let executable = directory.join(format!("{candidate}.exe"));
            #[cfg(not(windows))]
            let executable = directory.join(candidate);

            if executable.is_file() {
                return Some(executable);
            }
        }
    }

    None
}

fn set_setting(connection: &rusqlite::Connection, key: &str, value: &str) -> AppResult<()> {
    connection.execute(
        "INSERT INTO app_settings (key, value, updated_at)
     VALUES (?1, ?2, ?3)
     ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, utc_now()],
    )?;
    Ok(())
}

fn get_setting(connection: &rusqlite::Connection, key: &str, default: &str) -> AppResult<String> {
    let value = connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    Ok(value.unwrap_or_else(|| default.to_string()))
}

fn reporting_currency_label(connection: &rusqlite::Connection) -> AppResult<String> {
    let value = get_setting(connection, "reporting_currency_label", "")?;
    if !value.trim().is_empty() {
        return Ok(value.trim().to_string());
    }

    let legacy = get_setting(connection, "default_currency_label", "CAD")?;
    if legacy.trim().is_empty() {
        Ok("CAD".to_string())
    } else {
        Ok(legacy.trim().to_string())
    }
}

fn next_invoice_number(transaction: &rusqlite::Transaction<'_>) -> AppResult<String> {
    let prefix = get_setting(transaction, "invoice_number_prefix", "INV")?;
    let sequence_value = get_setting(transaction, "invoice_sequence", "1")?;
    let sequence = sequence_value.parse::<i64>().unwrap_or(1).max(1);
    let invoice_number = format!("{prefix}-{sequence:06}");
    set_setting(transaction, "invoice_sequence", &(sequence + 1).to_string())?;
    Ok(invoice_number)
}

fn ensure_invoice_number_unique(
    connection: &rusqlite::Connection,
    invoice_number: &str,
) -> AppResult<()> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM invoices WHERE invoice_number = ?1 LIMIT 1",
            params![invoice_number],
            |_| Ok(()),
        )
        .optional()?;

    if exists.is_some() {
        Err(AppError::Conflict(format!(
            "invoice number {invoice_number} already exists"
        )))
    } else {
        Ok(())
    }
}

fn calc_line_total_minor(quantity: &str, rate_minor: i64) -> AppResult<i64> {
    let quantity_decimal = parse_decimal(quantity, "quantity")?;
    let rate_decimal = Decimal::from_i64(rate_minor)
        .ok_or_else(|| AppError::Validation("rate is out of range".to_string()))?;
    let total = quantity_decimal * rate_decimal;
    let rounded = total.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
    rounded
        .to_i64()
        .ok_or_else(|| AppError::Validation("line total is out of range".to_string()))
}

fn business_snapshot_from_profile(profile: &BusinessProfile) -> BusinessSnapshot {
    BusinessSnapshot {
        business_id: profile.id,
        business_name: profile.business_name.clone(),
        legal_name: profile.legal_name.clone(),
        address: profile.address.clone(),
        country: profile.country.clone(),
        email: profile.email.clone(),
        phone: profile.phone.clone(),
        registration_number: profile.registration_number.clone(),
        tax_vat_number: profile.tax_vat_number.clone(),
        logo_path: profile.logo_path.clone(),
        captured_at: utc_now(),
    }
}

fn client_snapshot_from_client(client: &Client) -> ClientSnapshot {
    ClientSnapshot {
        client_id: client.id,
        company_name: client.company_name.clone(),
        contact_person: client.contact_person.clone(),
        email: client.email.clone(),
        address: client.address.clone(),
        country: client.country.clone(),
        notes: client.notes.clone(),
        captured_at: utc_now(),
    }
}

fn totals_snapshot(
    subtotal_minor: i64,
    total_minor: i64,
    paid_minor: i64,
    outstanding_minor: i64,
    currency_label: &str,
) -> InvoiceTotalsSnapshot {
    InvoiceTotalsSnapshot {
        subtotal_minor,
        total_minor,
        paid_minor,
        outstanding_minor,
        currency_label: currency_label.to_string(),
        captured_at: utc_now(),
    }
}

fn status_from_invoice_balance(
    invoice: &InvoiceRecord,
    paid_minor: i64,
    outstanding_minor: i64,
) -> InvoiceStatus {
    if matches!(invoice.status, InvoiceStatus::Cancelled) {
        return InvoiceStatus::Cancelled;
    }

    if matches!(invoice.status, InvoiceStatus::Draft) {
        return InvoiceStatus::Draft;
    }

    if paid_minor >= invoice.total_minor && invoice.total_minor > 0 {
        InvoiceStatus::Paid
    } else if paid_minor > 0 {
        InvoiceStatus::PartiallyPaid
    } else if outstanding_minor > 0 && invoice.due_date < today_date() {
        InvoiceStatus::Overdue
    } else {
        InvoiceStatus::Sent
    }
}

fn row_to_business_profile(row: &Row<'_>) -> rusqlite::Result<BusinessProfile> {
    Ok(BusinessProfile {
        id: row.get("id")?,
        business_name: row.get("business_name")?,
        legal_name: row.get("legal_name")?,
        address: row.get("address")?,
        country: row.get("country")?,
        email: row.get("email")?,
        phone: row.get("phone")?,
        registration_number: row.get("registration_number")?,
        tax_vat_number: row.get("tax_vat_number")?,
        logo_path: row.get("logo_path")?,
        is_active: row.get::<_, i64>("is_active")? == 1,
        archived_at: row.get("archived_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_client(row: &Row<'_>) -> rusqlite::Result<Client> {
    Ok(Client {
        id: row.get("id")?,
        company_name: row.get("company_name")?,
        contact_person: row.get("contact_person")?,
        email: row.get("email")?,
        address: row.get("address")?,
        country: row.get("country")?,
        notes: row.get("notes")?,
        archived_at: row.get("archived_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_invoice_summary(row: &Row<'_>) -> rusqlite::Result<InvoiceSummary> {
    let status_value: String = row.get("status")?;
    let status = InvoiceStatus::from_db(&status_value)
        .ok_or_else(|| rusqlite::Error::InvalidColumnType(0, "status".to_string(), Type::Text))?;

    Ok(InvoiceSummary {
        id: row.get("id")?,
        invoice_number: row.get("invoice_number")?,
        client_id: row.get("client_id")?,
        client_company_name: row.get("client_company_name")?,
        status,
        issue_date: row.get("issue_date")?,
        due_date: row.get("due_date")?,
        currency_label: row.get("currency_label")?,
        total_minor: row.get("total_minor")?,
        paid_minor: row.get("paid_minor")?,
        outstanding_minor: row.get("outstanding_minor")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_payment(row: &Row<'_>) -> rusqlite::Result<Payment> {
    let source_value: String = row.get("payment_source")?;
    let source = PaymentSource::from_db(&source_value).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(0, "payment_source".to_string(), Type::Text)
    })?;

    Ok(Payment {
        id: row.get("id")?,
        invoice_id: row.get("invoice_id")?,
        invoice_number: row.get("invoice_number")?,
        client_company_name: row.get("client_company_name")?,
        amount_minor: row.get("amount_minor")?,
        currency_label: row.get("currency_label")?,
        converted_amount_minor: row.get("converted_amount_minor")?,
        reporting_currency_label: row.get("reporting_currency_label")?,
        conversion_rate: row.get("conversion_rate")?,
        payment_date: row.get("payment_date")?,
        payment_source: source,
        transaction_reference_id: row.get("transaction_reference_id")?,
        notes: row.get("notes")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_currency_conversion(row: &Row<'_>) -> rusqlite::Result<CurrencyConversion> {
    Ok(CurrencyConversion {
        id: row.get("id")?,
        invoice_id: row.get("invoice_id")?,
        payment_id: row.get("payment_id")?,
        source_currency_label: row.get("source_currency_label")?,
        target_currency_label: row.get("target_currency_label")?,
        conversion_rate: row.get("conversion_rate")?,
        source_amount_minor: row.get("source_amount_minor")?,
        converted_amount_minor: row.get("converted_amount_minor")?,
        captured_at: row.get("captured_at")?,
    })
}

fn row_to_line_item(row: &Row<'_>) -> rusqlite::Result<InvoiceLineItem> {
    Ok(InvoiceLineItem {
        id: row.get("id")?,
        invoice_id: row.get("invoice_id")?,
        position: row.get("position")?,
        description: row.get("description")?,
        quantity: row.get("quantity")?,
        rate_minor: row.get("rate_minor")?,
        line_total_minor: row.get("line_total_minor")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn json_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
}

fn row_to_invoice_record(row: &Row<'_>) -> rusqlite::Result<InvoiceRecord> {
    let status_str: String = row.get("status")?;
    let status = InvoiceStatus::from_db(&status_str)
        .ok_or_else(|| rusqlite::Error::InvalidColumnType(0, "status".to_string(), Type::Text))?;
    let business_snapshot_json: String = row.get("business_snapshot_json")?;
    let client_snapshot_json: String = row.get("client_snapshot_json")?;
    let totals_snapshot_json: String = row.get("totals_snapshot_json")?;
    let conversion_snapshot_json: String = row.get("conversion_snapshot_json")?;

    Ok(InvoiceRecord {
        id: row.get("id")?,
        invoice_number: row.get("invoice_number")?,
        business_id: row.get("business_id")?,
        client_id: row.get("client_id")?,
        status,
        issue_date: row.get("issue_date")?,
        due_date: row.get("due_date")?,
        currency_label: row.get("currency_label")?,
        notes: row.get("notes")?,
        payment_terms: row.get("payment_terms")?,
        subtotal_minor: row.get("subtotal_minor")?,
        total_minor: row.get("total_minor")?,
        paid_minor: row.get("paid_minor")?,
        outstanding_minor: row.get("outstanding_minor")?,
        finalized_at: row.get("finalized_at")?,
        locked_at: row.get("locked_at")?,
        cancelled_at: row.get("cancelled_at")?,
        business_snapshot: serde_json::from_str(&business_snapshot_json).map_err(json_sql_error)?,
        client_snapshot: serde_json::from_str(&client_snapshot_json).map_err(json_sql_error)?,
        totals_snapshot: serde_json::from_str(&totals_snapshot_json).map_err(json_sql_error)?,
        conversion_snapshot: serde_json::from_str(&conversion_snapshot_json)
            .map_err(json_sql_error)?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn load_invoice_detail(
    connection: &rusqlite::Connection,
    invoice_id: i64,
) -> AppResult<InvoiceDetail> {
    let invoice_row = connection
        .query_row(
            "SELECT * FROM invoices WHERE id = ?1",
            params![invoice_id],
            row_to_invoice_record,
        )
        .optional()?;

    let invoice =
        invoice_row.ok_or_else(|| AppError::NotFound(format!("invoice {invoice_id} not found")))?;

    let mut line_items_statement = connection.prepare(
        "SELECT * FROM invoice_line_items WHERE invoice_id = ?1 ORDER BY position ASC, id ASC",
    )?;
    let line_items = line_items_statement
        .query_map(params![invoice_id], row_to_line_item)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut payments_statement = connection.prepare(
        r#"
      SELECT p.*, i.invoice_number, c.company_name AS client_company_name,
             cc.target_currency_label AS reporting_currency_label,
             cc.conversion_rate AS conversion_rate
      FROM payments p
      JOIN invoices i ON i.id = p.invoice_id
      JOIN clients c ON c.id = i.client_id
      LEFT JOIN currency_conversions cc ON cc.payment_id = p.id
      WHERE p.invoice_id = ?1
      ORDER BY p.payment_date DESC, p.created_at DESC, p.id DESC
    "#,
    )?;
    let payments = payments_statement
        .query_map(params![invoice_id], row_to_payment)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut conversions_statement = connection.prepare(
    "SELECT * FROM currency_conversions WHERE invoice_id = ?1 ORDER BY captured_at DESC, id DESC",
  )?;
    let conversions = conversions_statement
        .query_map(params![invoice_id], row_to_currency_conversion)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(InvoiceDetail {
        invoice,
        line_items,
        payments,
        conversions,
    })
}

fn refresh_invoice_summary(
    connection: &rusqlite::Connection,
    invoice_id: i64,
) -> AppResult<InvoiceRecord> {
    let detail = load_invoice_detail(connection, invoice_id)?;
    let invoice = detail.invoice;
    let is_locked = invoice.locked_at.is_some();

    let paid_minor = detail
        .payments
        .iter()
        .map(|payment| payment.amount_minor)
        .sum::<i64>();
    let outstanding_minor = (invoice.total_minor - paid_minor).max(0);
    let status = status_from_invoice_balance(&invoice, paid_minor, outstanding_minor);
    let conversion_snapshot_json = serde_json::to_string(&detail.conversions)?;
    let totals_snapshot = totals_snapshot(
        invoice.subtotal_minor,
        invoice.total_minor,
        paid_minor,
        outstanding_minor,
        &invoice.currency_label,
    );

    if is_locked {
        connection.execute(
            "UPDATE invoices
         SET paid_minor = ?2,
             outstanding_minor = ?3,
             status = ?4,
             updated_at = ?5
         WHERE id = ?1",
            params![
                invoice_id,
                paid_minor,
                outstanding_minor,
                status.as_str(),
                utc_now()
            ],
        )?;
    } else {
        connection.execute(
            "UPDATE invoices
         SET paid_minor = ?2,
             outstanding_minor = ?3,
             status = ?4,
             totals_snapshot_json = ?5,
             conversion_snapshot_json = ?6,
             updated_at = ?7
         WHERE id = ?1",
            params![
                invoice_id,
                paid_minor,
                outstanding_minor,
                status.as_str(),
                serde_json::to_string(&totals_snapshot)?,
                conversion_snapshot_json,
                utc_now()
            ],
        )?;
    }

    let updated = connection.query_row(
        "SELECT * FROM invoices WHERE id = ?1",
        params![invoice_id],
        row_to_invoice_record,
    )?;

    Ok(updated)
}

fn render_logo_data_uri(path: &Option<String>) -> Option<String> {
    let logo_path = path.as_ref()?;
    let bytes = fs::read(logo_path).ok()?;
    let encoded = general_purpose::STANDARD.encode(bytes);
    let extension = Path::new(logo_path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png")
        .to_lowercase();
    let mime_type = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => "image/png",
    };

    Some(format!("data:{mime_type};base64,{encoded}"))
}

pub fn get_app_state(database: &Database) -> AppResult<AppBootState> {
    let connection = database.open()?;
    let settings = load_settings_map(&connection)?;
    let active_business = get_active_business_record(&connection)?;
    let reporting_currency_label = reporting_currency_label(&connection)?;

    Ok(AppBootState {
        database_path: database.path().to_string_lossy().to_string(),
        active_business,
        reporting_currency_label,
        settings,
    })
}

pub fn load_settings_map(connection: &rusqlite::Connection) -> AppResult<HashMap<String, String>> {
    let mut statement =
        connection.prepare("SELECT key, value FROM app_settings ORDER BY key ASC")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut settings = HashMap::new();
    for row in rows {
        let (key, value) = row?;
        settings.insert(key, value);
    }

    Ok(settings)
}

pub fn update_settings(
    database: &Database,
    settings: HashMap<String, String>,
) -> AppResult<HashMap<String, String>> {
    let connection = database.open()?;
    for (key, value) in settings {
        set_setting(&connection, &key, &value)?;
    }
    load_settings_map(&connection)
}

pub fn get_active_business_record(
    connection: &rusqlite::Connection,
) -> AppResult<Option<BusinessProfile>> {
    let profile = connection
        .query_row(
            "SELECT * FROM businesses WHERE is_active = 1 AND archived_at IS NULL LIMIT 1",
            [],
            row_to_business_profile,
        )
        .optional()?;
    Ok(profile)
}

pub fn list_businesses(
    database: &Database,
    include_archived: bool,
) -> AppResult<Vec<BusinessProfile>> {
    let connection = database.open()?;
    let mut statement = connection.prepare(
        "SELECT * FROM businesses
     WHERE (?1 = 1 OR archived_at IS NULL)
     ORDER BY is_active DESC, business_name ASC, id ASC",
    )?;
    let rows = statement.query_map(
        params![i64::from(include_archived)],
        row_to_business_profile,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn create_business(
    database: &Database,
    input: BusinessProfileInput,
) -> AppResult<BusinessProfile> {
    validate_required("business name", &input.business_name)?;
    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    if input.is_active {
        transaction.execute(
            "UPDATE businesses SET is_active = 0, updated_at = ?1 WHERE is_active = 1",
            params![utc_now()],
        )?;
    }

    let should_activate = input.is_active || get_active_business_record(&transaction)?.is_none();
    transaction.execute(
        "INSERT INTO businesses (
      is_active, business_name, legal_name, address, country, email, phone,
      registration_number, tax_vat_number, logo_path, archived_at, created_at, updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11, ?12)",
        params![
            if should_activate { 1 } else { 0 },
            input.business_name.trim(),
            input.legal_name,
            input.address,
            input.country,
            input.email,
            input.phone,
            input.registration_number,
            input.tax_vat_number,
            input.logo_path,
            utc_now(),
            utc_now()
        ],
    )?;

    let id = transaction.last_insert_rowid();
    transaction.commit()?;
    get_business(database, id)
}

pub fn get_business(database: &Database, id: i64) -> AppResult<BusinessProfile> {
    let connection = database.open()?;
    let profile = connection.query_row(
        "SELECT * FROM businesses WHERE id = ?1",
        params![id],
        row_to_business_profile,
    )?;
    Ok(profile)
}

pub fn update_business(
    database: &Database,
    id: i64,
    input: BusinessProfileInput,
) -> AppResult<BusinessProfile> {
    validate_required("business name", &input.business_name)?;
    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    let existing = transaction
        .query_row(
            "SELECT * FROM businesses WHERE id = ?1",
            params![id],
            row_to_business_profile,
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("business {id} not found")))?;
    if input.is_active {
        transaction.execute(
            "UPDATE businesses SET is_active = 0, updated_at = ?1 WHERE is_active = 1 AND id != ?2",
            params![utc_now(), id],
        )?;
    }
    transaction.execute(
        "UPDATE businesses
     SET business_name = ?2,
         legal_name = ?3,
         address = ?4,
         country = ?5,
         email = ?6,
         phone = ?7,
         registration_number = ?8,
         tax_vat_number = ?9,
         logo_path = ?10,
         is_active = ?11,
         archived_at = ?12,
         updated_at = ?13
     WHERE id = ?1",
        params![
            id,
            input.business_name.trim(),
            input.legal_name,
            input.address,
            input.country,
            input.email,
            input.phone,
            input.registration_number,
            input.tax_vat_number,
            input.logo_path,
            if input.is_active { 1 } else { 0 },
            existing.archived_at,
            utc_now()
        ],
    )?;

    transaction.commit()?;
    get_business(database, id)
}

pub fn archive_business(database: &Database, id: i64) -> AppResult<BusinessProfile> {
    let connection = database.open()?;
    connection.execute(
        "UPDATE businesses SET archived_at = ?2, is_active = 0, updated_at = ?3 WHERE id = ?1",
        params![id, utc_now(), utc_now()],
    )?;
    get_business(database, id)
}

pub fn set_active_business(database: &Database, id: i64) -> AppResult<BusinessProfile> {
    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    let archived = transaction
        .query_row(
            "SELECT archived_at FROM businesses WHERE id = ?1",
            params![id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?;

    let archived =
        archived.ok_or_else(|| AppError::NotFound(format!("business {id} not found")))?;

    if archived.is_some() {
        return Err(AppError::Conflict(
            "archived businesses cannot be activated".to_string(),
        ));
    }

    transaction.execute(
        "UPDATE businesses SET is_active = 0, updated_at = ?1 WHERE is_active = 1",
        params![utc_now()],
    )?;
    transaction.execute(
        "UPDATE businesses SET is_active = 1, updated_at = ?2 WHERE id = ?1",
        params![id, utc_now()],
    )?;
    transaction.commit()?;
    get_business(database, id)
}

pub fn list_clients(
    database: &Database,
    search: Option<String>,
    include_archived: bool,
) -> AppResult<Vec<Client>> {
    let connection = database.open()?;
    let search_pattern = search.as_ref().map(|value| format!("%{}%", value.trim()));
    let mut statement = connection.prepare(
        "SELECT * FROM clients
     WHERE (:include_archived = 1 OR archived_at IS NULL)
       AND (
         :search IS NULL
         OR company_name LIKE :search
         OR IFNULL(contact_person, '') LIKE :search
         OR IFNULL(email, '') LIKE :search
       )
     ORDER BY archived_at IS NOT NULL, company_name ASC, id ASC",
    )?;
    let rows = statement.query_map(
        named_params! {
          ":search": search_pattern.as_deref(),
          ":include_archived": i64::from(include_archived),
        },
        row_to_client,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_client(database: &Database, id: i64) -> AppResult<ClientDetail> {
    let connection = database.open()?;
    let client = connection.query_row(
        "SELECT * FROM clients WHERE id = ?1",
        params![id],
        row_to_client,
    )?;

    let mut statement = connection.prepare(
        r#"
      SELECT i.id, i.invoice_number, i.client_id, c.company_name AS client_company_name,
             i.status, i.issue_date, i.due_date, i.currency_label,
             i.total_minor, i.paid_minor, i.outstanding_minor,
             i.created_at, i.updated_at
      FROM invoices i
      JOIN clients c ON c.id = i.client_id
      WHERE i.client_id = ?1
      ORDER BY i.issue_date DESC, i.created_at DESC, i.id DESC
    "#,
    )?;
    let invoice_history = statement
        .query_map(params![id], row_to_invoice_summary)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ClientDetail {
        client,
        invoice_history,
    })
}

pub fn create_client(database: &Database, input: ClientInput) -> AppResult<Client> {
    validate_required("client/company name", &input.company_name)?;
    let connection = database.open()?;
    connection.execute(
    "INSERT INTO clients (
      company_name, contact_person, email, address, country, notes, archived_at, created_at, updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8)",
    params![
      input.company_name.trim(),
      input.contact_person,
      input.email,
      input.address,
      input.country,
      input.notes,
      utc_now(),
      utc_now(),
    ],
  )?;
    let id = connection.last_insert_rowid();
    get_client_record(database, id)
}

fn get_client_record(database: &Database, id: i64) -> AppResult<Client> {
    let connection = database.open()?;
    let client = connection.query_row(
        "SELECT * FROM clients WHERE id = ?1",
        params![id],
        row_to_client,
    )?;
    Ok(client)
}

pub fn update_client(database: &Database, id: i64, input: ClientInput) -> AppResult<Client> {
    validate_required("client/company name", &input.company_name)?;
    let connection = database.open()?;
    connection.execute(
        "UPDATE clients
     SET company_name = ?2,
         contact_person = ?3,
         email = ?4,
         address = ?5,
         country = ?6,
         notes = ?7,
         updated_at = ?8
     WHERE id = ?1",
        params![
            id,
            input.company_name.trim(),
            input.contact_person,
            input.email,
            input.address,
            input.country,
            input.notes,
            utc_now(),
        ],
    )?;
    get_client_record(database, id)
}

pub fn archive_client(database: &Database, id: i64) -> AppResult<Client> {
    let connection = database.open()?;
    connection.execute(
        "UPDATE clients SET archived_at = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, utc_now(), utc_now()],
    )?;
    get_client_record(database, id)
}

pub fn delete_client(database: &Database, id: i64) -> AppResult<()> {
    let connection = database.open()?;
    let invoice_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM invoices WHERE client_id = ?1",
        params![id],
        |row| row.get(0),
    )?;

    if invoice_count > 0 {
        return Err(AppError::Conflict(
            "clients with invoice history should be archived, not deleted".to_string(),
        ));
    }

    connection.execute("DELETE FROM clients WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn create_invoice(database: &Database, input: InvoiceFormInput) -> AppResult<InvoiceDetail> {
    validate_required("invoice currency label", &input.currency_label)?;
    validate_required("issue date", &input.issue_date)?;
    validate_required("due date", &input.due_date)?;
    if input.line_items.is_empty() {
        return Err(AppError::Validation(
            "an invoice must contain at least one line item".to_string(),
        ));
    }

    let mut connection = database.open()?;
    let transaction = connection.transaction()?;

    let business_id = match input.business_id {
        Some(id) => id,
        None => {
            get_active_business_record(&transaction)?
                .ok_or_else(|| {
                    AppError::Validation(
                        "create a business profile before creating invoices".to_string(),
                    )
                })?
                .id
        }
    };
    let business = transaction.query_row(
        "SELECT * FROM businesses WHERE id = ?1",
        params![business_id],
        row_to_business_profile,
    )?;
    let client = transaction.query_row(
        "SELECT * FROM clients WHERE id = ?1",
        params![input.client_id],
        row_to_client,
    )?;

    let invoice_number = match input.invoice_number {
        Some(value) => {
            validate_required("invoice number", &value)?;
            ensure_invoice_number_unique(&transaction, &value)?;
            value.trim().to_string()
        }
        None => next_invoice_number(&transaction)?,
    };

    let line_item_totals = input
        .line_items
        .iter()
        .map(|item| calc_line_total_minor(&item.quantity, item.rate_minor))
        .collect::<AppResult<Vec<_>>>()?;
    let subtotal_minor = line_item_totals.iter().copied().sum::<i64>();
    let total_minor = subtotal_minor;
    let paid_minor = 0;
    let outstanding_minor = total_minor;
    let invoice_status = InvoiceStatus::Draft;
    let business_snapshot = business_snapshot_from_profile(&business);
    let client_snapshot = client_snapshot_from_client(&client);
    let totals_snapshot = totals_snapshot(
        subtotal_minor,
        total_minor,
        paid_minor,
        outstanding_minor,
        &input.currency_label,
    );

    transaction.execute(
    "INSERT INTO invoices (
      invoice_number, business_id, client_id, status, issue_date, due_date,
      currency_label, notes, payment_terms, subtotal_minor, total_minor, paid_minor,
      outstanding_minor, finalized_at, locked_at, cancelled_at,
      business_snapshot_json, client_snapshot_json, totals_snapshot_json, conversion_snapshot_json,
      created_at, updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, NULL, NULL, ?14, ?15, ?16, '[]', ?17, ?18)",
    params![
      invoice_number,
      business_id,
      input.client_id,
      invoice_status.as_str(),
      input.issue_date,
      input.due_date,
      input.currency_label.trim(),
      input.notes,
      input.payment_terms,
      subtotal_minor,
      total_minor,
      paid_minor,
      outstanding_minor,
      serde_json::to_string(&business_snapshot)?,
      serde_json::to_string(&client_snapshot)?,
      serde_json::to_string(&totals_snapshot)?,
      utc_now(),
      utc_now(),
    ],
  )?;

    let invoice_id = transaction.last_insert_rowid();
    insert_line_items(
        &transaction,
        invoice_id,
        &input.line_items,
        &line_item_totals,
    )?;
    refresh_invoice_summary(&transaction, invoice_id)?;
    transaction.commit()?;

    get_invoice(database, invoice_id)
}

fn insert_line_items(
    connection: &rusqlite::Transaction<'_>,
    invoice_id: i64,
    line_items: &[InvoiceLineItemInput],
    line_item_totals: &[i64],
) -> AppResult<()> {
    for (index, item) in line_items.iter().enumerate() {
        connection.execute(
      "INSERT INTO invoice_line_items (
        invoice_id, position, description, quantity, rate_minor, line_total_minor, created_at, updated_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      params![
        invoice_id,
        (index + 1) as i64,
        item.description.trim(),
        item.quantity.trim(),
        item.rate_minor,
        line_item_totals[index],
        utc_now(),
        utc_now()
      ],
    )?;
    }
    Ok(())
}

pub fn update_invoice(
    database: &Database,
    id: i64,
    input: InvoiceFormInput,
) -> AppResult<InvoiceDetail> {
    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    let existing = load_invoice_detail(&transaction, id)?;

    if !matches!(existing.invoice.status, InvoiceStatus::Draft) {
        return Err(AppError::Conflict(
            "only draft invoices can be edited".to_string(),
        ));
    }

    if let Some(provided_invoice_number) = input.invoice_number.as_ref() {
        validate_required("invoice number", provided_invoice_number)?;
        if provided_invoice_number.trim() != existing.invoice.invoice_number {
            ensure_invoice_number_unique(&transaction, provided_invoice_number.trim())?;
        }
    }

    let business_id = input.business_id.unwrap_or(existing.invoice.business_id);
    let business = transaction.query_row(
        "SELECT * FROM businesses WHERE id = ?1",
        params![business_id],
        row_to_business_profile,
    )?;
    let client = transaction.query_row(
        "SELECT * FROM clients WHERE id = ?1",
        params![input.client_id],
        row_to_client,
    )?;

    let line_item_totals = input
        .line_items
        .iter()
        .map(|item| calc_line_total_minor(&item.quantity, item.rate_minor))
        .collect::<AppResult<Vec<_>>>()?;
    let subtotal_minor = line_item_totals.iter().copied().sum::<i64>();
    let total_minor = subtotal_minor;
    let outstanding_minor = total_minor;
    let business_snapshot = if business_id == existing.invoice.business_id {
        existing.invoice.business_snapshot.clone()
    } else {
        business_snapshot_from_profile(&business)
    };
    let client_snapshot = if input.client_id == existing.invoice.client_id {
        existing.invoice.client_snapshot.clone()
    } else {
        client_snapshot_from_client(&client)
    };
    let totals_snapshot = totals_snapshot(
        subtotal_minor,
        total_minor,
        0,
        outstanding_minor,
        &input.currency_label,
    );

    transaction.execute(
        "UPDATE invoices
     SET invoice_number = ?2,
         business_id = ?3,
         client_id = ?4,
         issue_date = ?5,
         due_date = ?6,
         currency_label = ?7,
         notes = ?8,
         payment_terms = ?9,
         subtotal_minor = ?10,
         total_minor = ?11,
         paid_minor = 0,
         outstanding_minor = ?12,
         business_snapshot_json = ?13,
         client_snapshot_json = ?14,
         totals_snapshot_json = ?15,
         updated_at = ?16
     WHERE id = ?1",
        params![
            id,
            input
                .invoice_number
                .unwrap_or_else(|| existing.invoice.invoice_number.clone())
                .trim()
                .to_string(),
            business_id,
            input.client_id,
            input.issue_date,
            input.due_date,
            input.currency_label.trim(),
            input.notes,
            input.payment_terms,
            subtotal_minor,
            total_minor,
            outstanding_minor,
            serde_json::to_string(&business_snapshot)?,
            serde_json::to_string(&client_snapshot)?,
            serde_json::to_string(&totals_snapshot)?,
            utc_now(),
        ],
    )?;

    transaction.execute(
        "DELETE FROM invoice_line_items WHERE invoice_id = ?1",
        params![id],
    )?;
    insert_line_items(&transaction, id, &input.line_items, &line_item_totals)?;
    refresh_invoice_summary(&transaction, id)?;
    transaction.commit()?;

    get_invoice(database, id)
}

pub fn delete_invoice(database: &Database, id: i64) -> AppResult<()> {
    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    let invoice_exists: Option<i64> = transaction
        .query_row(
            "SELECT id FROM invoices WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;

    if invoice_exists.is_none() {
        return Err(AppError::NotFound(format!("invoice {id} not found")));
    }

    let payment_count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM payments WHERE invoice_id = ?1",
        params![id],
        |row| row.get(0),
    )?;

    if payment_count > 0 {
        return Err(AppError::Conflict(format!(
            "delete {payment_count} payment(s) first before deleting this invoice"
        )));
    }

    transaction.execute("DELETE FROM invoices WHERE id = ?1", params![id])?;
    transaction.commit()?;
    Ok(())
}

pub fn delete_draft_invoice(database: &Database, id: i64) -> AppResult<()> {
    delete_invoice(database, id)
}

pub fn duplicate_invoice(database: &Database, id: i64) -> AppResult<InvoiceDetail> {
    let original = get_invoice(database, id)?;
    create_invoice(
        database,
        InvoiceFormInput {
            business_id: Some(original.invoice.business_id),
            client_id: original.invoice.client_id,
            invoice_number: None,
            issue_date: today_date(),
            due_date: original.invoice.due_date.clone(),
            currency_label: original.invoice.currency_label.clone(),
            notes: original.invoice.notes.clone(),
            payment_terms: original.invoice.payment_terms.clone(),
            line_items: original
                .line_items
                .iter()
                .map(|item| InvoiceLineItemInput {
                    description: item.description.clone(),
                    quantity: item.quantity.clone(),
                    rate_minor: item.rate_minor,
                })
                .collect(),
        },
    )
}

pub fn finalize_invoice(database: &Database, id: i64) -> AppResult<InvoiceDetail> {
    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    let existing = load_invoice_detail(&transaction, id)?;

    if !matches!(existing.invoice.status, InvoiceStatus::Draft) {
        return Err(AppError::Conflict(
            "only draft invoices can be finalized".to_string(),
        ));
    }

    refresh_invoice_summary(&transaction, id)?;
    let refreshed = load_invoice_detail(&transaction, id)?;
    let base_status =
        if refreshed.invoice.outstanding_minor > 0 && refreshed.invoice.due_date < today_date() {
            InvoiceStatus::Overdue
        } else {
            InvoiceStatus::Sent
        };
    transaction.execute(
        "UPDATE invoices
     SET status = ?2,
         finalized_at = ?3,
         locked_at = ?4,
         updated_at = ?5
     WHERE id = ?1",
        params![id, base_status.as_str(), utc_now(), utc_now(), utc_now()],
    )?;
    refresh_invoice_summary(&transaction, id)?;
    transaction.commit()?;

    get_invoice(database, id)
}

pub fn reorder_invoice_line_items(
    database: &Database,
    invoice_id: i64,
    ordered_line_item_ids: Vec<i64>,
) -> AppResult<()> {
    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    let status: String = transaction.query_row(
        "SELECT status FROM invoices WHERE id = ?1",
        params![invoice_id],
        |row| row.get(0),
    )?;

    if status != InvoiceStatus::Draft.as_str() {
        return Err(AppError::Conflict(
            "only draft invoices can be reordered".to_string(),
        ));
    }

    for (position, line_item_id) in ordered_line_item_ids.iter().enumerate() {
        transaction.execute(
            "UPDATE invoice_line_items SET position = ?2, updated_at = ?3 WHERE id = ?1 AND invoice_id = ?4",
            params![line_item_id, (position + 1) as i64, utc_now(), invoice_id],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

pub fn get_invoice(database: &Database, invoice_id: i64) -> AppResult<InvoiceDetail> {
    let connection = database.open()?;
    load_invoice_detail(&connection, invoice_id)
}

pub fn list_invoices(
    database: &Database,
    filters: InvoiceFilters,
) -> AppResult<Vec<InvoiceSummary>> {
    let connection = database.open()?;
    let search_pattern = filters
        .search
        .as_ref()
        .map(|value| format!("%{}%", value.trim()));
    let status_value = filters.status.map(|value| value.as_str().to_string());
    let mut statement = connection.prepare(
        r#"
      SELECT i.id, i.invoice_number, i.client_id, c.company_name AS client_company_name,
             i.status, i.issue_date, i.due_date, i.currency_label,
             i.total_minor, i.paid_minor, i.outstanding_minor,
             i.created_at, i.updated_at
      FROM invoices i
      JOIN clients c ON c.id = i.client_id
      WHERE (
        :search IS NULL
        OR i.invoice_number LIKE :search
        OR c.company_name LIKE :search
        OR IFNULL(i.notes, '') LIKE :search
      )
      AND (:client_id IS NULL OR i.client_id = :client_id)
      AND (:currency_label IS NULL OR i.currency_label = :currency_label)
      AND (:status IS NULL OR i.status = :status)
      AND (:from_date IS NULL OR i.issue_date >= :from_date)
      AND (:to_date IS NULL OR i.issue_date <= :to_date)
      ORDER BY i.issue_date DESC, i.created_at DESC, i.id DESC
    "#,
    )?;
    let rows = statement.query_map(
        named_params! {
          ":search": search_pattern.as_deref(),
          ":client_id": filters.client_id,
          ":currency_label": filters.currency_label.as_deref(),
          ":status": status_value.as_deref(),
          ":from_date": filters.from_date.as_deref(),
          ":to_date": filters.to_date.as_deref(),
        },
        row_to_invoice_summary,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn record_payment(database: &Database, input: PaymentInput) -> AppResult<Payment> {
    if input.amount_minor <= 0 {
        return Err(AppError::Validation(
            "payment amount must be positive".to_string(),
        ));
    }

    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    let invoice: InvoiceRecord = load_invoice_detail(&transaction, input.invoice_id)?.invoice;

    if matches!(
        invoice.status,
        InvoiceStatus::Draft | InvoiceStatus::Cancelled
    ) || invoice.outstanding_minor <= 0
    {
        return Err(AppError::Conflict(
            "record payments against invoices with an outstanding balance only".to_string(),
        ));
    }
    if input.currency_label.trim() != invoice.currency_label {
        return Err(AppError::Conflict(
            "payment currency must match the invoice currency; use the reporting snapshot for bookkeeping totals".to_string(),
        ));
    }

    let target_currency = input
        .reporting_currency_label
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(reporting_currency_label(&transaction)?);
    let converted_amount_minor = match (input.converted_amount_minor, input.conversion_rate.as_ref()) {
    (Some(value), _) => value,
    (None, Some(_rate)) if input.currency_label == target_currency => input.amount_minor,
    (None, Some(rate)) => {
      let rate_decimal = parse_decimal(rate, "conversion rate")?;
      let source_amount = Decimal::from_i64(input.amount_minor)
        .ok_or_else(|| AppError::Validation("payment amount is out of range".to_string()))?;
      (source_amount * rate_decimal)
        .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
        .to_i64()
        .ok_or_else(|| AppError::Validation("converted amount is out of range".to_string()))?
    }
    (None, None) if input.currency_label == target_currency => input.amount_minor,
    (None, None) => {
      return Err(AppError::Validation(
        "a conversion rate or converted amount is required when payment currency differs from the reporting currency"
          .to_string(),
      ))
    }
  };

    let conversion_rate = input
        .conversion_rate
        .clone()
        .unwrap_or_else(|| "1".to_string());

    transaction.execute(
        "INSERT INTO payments (
      invoice_id, amount_minor, currency_label, converted_amount_minor, payment_date,
      payment_source, transaction_reference_id, notes, created_at, updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            input.invoice_id,
            input.amount_minor,
            input.currency_label.trim(),
            Some(converted_amount_minor),
            input.payment_date,
            input.payment_source.as_str(),
            input.transaction_reference_id,
            input.notes,
            utc_now(),
            utc_now(),
        ],
    )?;
    let payment_id = transaction.last_insert_rowid();

    transaction.execute(
        "INSERT INTO currency_conversions (
      invoice_id, payment_id, source_currency_label, target_currency_label,
      conversion_rate, source_amount_minor, converted_amount_minor, captured_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.invoice_id,
            payment_id,
            input.currency_label.trim(),
            target_currency,
            conversion_rate,
            input.amount_minor,
            converted_amount_minor,
            utc_now(),
        ],
    )?;

    refresh_invoice_summary(&transaction, input.invoice_id)?;
    transaction.commit()?;

    get_payment(database, payment_id)
}

pub fn update_payment(
    database: &Database,
    payment_id: i64,
    input: PaymentInput,
) -> AppResult<Payment> {
    if input.amount_minor <= 0 {
        return Err(AppError::Validation(
            "payment amount must be positive".to_string(),
        ));
    }

    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    let existing = get_payment(database, payment_id)?;
    if existing.invoice_id != input.invoice_id {
        return Err(AppError::Conflict(
            "moving payments between invoices is not supported yet".to_string(),
        ));
    }

    let invoice: InvoiceRecord = load_invoice_detail(&transaction, input.invoice_id)?.invoice;
    if matches!(
        invoice.status,
        InvoiceStatus::Draft | InvoiceStatus::Cancelled
    ) {
        return Err(AppError::Conflict(
            "record payments against active invoices only".to_string(),
        ));
    }
    if input.currency_label.trim() != invoice.currency_label {
        return Err(AppError::Conflict(
            "payment currency must match the invoice currency; use the reporting snapshot for bookkeeping totals".to_string(),
        ));
    }
    let target_currency = input
        .reporting_currency_label
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(reporting_currency_label(&transaction)?);
    let converted_amount_minor = match (input.converted_amount_minor, input.conversion_rate.as_ref()) {
    (Some(value), _) => value,
    (None, Some(_rate)) if input.currency_label == target_currency => input.amount_minor,
    (None, Some(rate)) => {
      let rate_decimal = parse_decimal(rate, "conversion rate")?;
      let source_amount = Decimal::from_i64(input.amount_minor)
        .ok_or_else(|| AppError::Validation("payment amount is out of range".to_string()))?;
      (source_amount * rate_decimal)
        .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
        .to_i64()
        .ok_or_else(|| AppError::Validation("converted amount is out of range".to_string()))?
    }
    (None, None) if input.currency_label == target_currency => input.amount_minor,
    (None, None) => {
      return Err(AppError::Validation(
        "a conversion rate or converted amount is required when payment currency differs from the reporting currency"
          .to_string(),
      ))
    }
  };

    transaction.execute(
        "UPDATE payments
     SET amount_minor = ?2,
         currency_label = ?3,
         converted_amount_minor = ?4,
         payment_date = ?5,
         payment_source = ?6,
         transaction_reference_id = ?7,
         notes = ?8,
         updated_at = ?9
     WHERE id = ?1",
        params![
            payment_id,
            input.amount_minor,
            input.currency_label.trim(),
            Some(converted_amount_minor),
            input.payment_date,
            input.payment_source.as_str(),
            input.transaction_reference_id,
            input.notes,
            utc_now(),
        ],
    )?;

    transaction.execute(
        "DELETE FROM currency_conversions WHERE payment_id = ?1",
        params![payment_id],
    )?;
    transaction.execute(
        "INSERT INTO currency_conversions (
      invoice_id, payment_id, source_currency_label, target_currency_label,
      conversion_rate, source_amount_minor, converted_amount_minor, captured_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.invoice_id,
            payment_id,
            input.currency_label.trim(),
            target_currency,
            input
                .conversion_rate
                .clone()
                .unwrap_or_else(|| "1".to_string()),
            input.amount_minor,
            converted_amount_minor,
            utc_now(),
        ],
    )?;

    refresh_invoice_summary(&transaction, input.invoice_id)?;
    transaction.commit()?;

    get_payment(database, payment_id)
}

pub fn delete_payment(database: &Database, payment_id: i64) -> AppResult<()> {
    let mut connection = database.open()?;
    let transaction = connection.transaction()?;
    let payment = get_payment(database, payment_id)?;
    transaction.execute(
        "DELETE FROM currency_conversions WHERE payment_id = ?1",
        params![payment_id],
    )?;
    transaction.execute("DELETE FROM payments WHERE id = ?1", params![payment_id])?;
    refresh_invoice_summary(&transaction, payment.invoice_id)?;
    transaction.commit()?;
    Ok(())
}

pub fn get_payment(database: &Database, payment_id: i64) -> AppResult<Payment> {
    let connection = database.open()?;
    let payment = connection.query_row(
        r#"
      SELECT p.*, i.invoice_number, c.company_name AS client_company_name,
             cc.target_currency_label AS reporting_currency_label,
             cc.conversion_rate AS conversion_rate
      FROM payments p
      JOIN invoices i ON i.id = p.invoice_id
      JOIN clients c ON c.id = i.client_id
      LEFT JOIN currency_conversions cc ON cc.payment_id = p.id
      WHERE p.id = ?1
    "#,
        params![payment_id],
        row_to_payment,
    )?;
    Ok(payment)
}

pub fn list_payments(database: &Database, filters: PaymentFilters) -> AppResult<Vec<Payment>> {
    let connection = database.open()?;
    let search_pattern = filters
        .search
        .as_ref()
        .map(|value| format!("%{}%", value.trim()));
    let mut statement = connection.prepare(
        r#"
      SELECT p.*, i.invoice_number, c.company_name AS client_company_name,
             cc.target_currency_label AS reporting_currency_label,
             cc.conversion_rate AS conversion_rate
      FROM payments p
      JOIN invoices i ON i.id = p.invoice_id
      JOIN clients c ON c.id = i.client_id
      LEFT JOIN currency_conversions cc ON cc.payment_id = p.id
      WHERE (
        :search IS NULL
        OR i.invoice_number LIKE :search
        OR c.company_name LIKE :search
        OR IFNULL(p.transaction_reference_id, '') LIKE :search
      )
      AND (:invoice_id IS NULL OR p.invoice_id = :invoice_id)
      AND (:client_id IS NULL OR i.client_id = :client_id)
      AND (:currency_label IS NULL OR p.currency_label = :currency_label)
      AND (:from_date IS NULL OR p.payment_date >= :from_date)
      AND (:to_date IS NULL OR p.payment_date <= :to_date)
      ORDER BY p.payment_date DESC, p.created_at DESC, p.id DESC
    "#,
    )?;
    let rows = statement.query_map(
        named_params! {
          ":search": search_pattern.as_deref(),
          ":invoice_id": filters.invoice_id,
          ":client_id": filters.client_id,
          ":currency_label": filters.currency_label.as_deref(),
          ":from_date": filters.from_date.as_deref(),
          ":to_date": filters.to_date.as_deref(),
        },
        row_to_payment,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn refresh_overdue_invoices(database: &Database) -> AppResult<()> {
    let connection = database.open()?;
    let mut statement = connection.prepare(
    "SELECT id FROM invoices WHERE status IN ('Sent', 'Partially Paid', 'Overdue') AND outstanding_minor > 0",
  )?;
    let ids = statement
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    for id in ids {
        refresh_invoice_summary(&connection, id)?;
    }

    Ok(())
}

pub fn get_dashboard_summary(
    database: &Database,
    filters: DashboardFilters,
) -> AppResult<DashboardSummary> {
    refresh_overdue_invoices(database)?;
    let connection = database.open()?;
    let reporting_currency = reporting_currency_label(&connection)?;
    let from_date = filters.from_date.as_deref();
    let to_date = filters.to_date.as_deref();
    let client_id = filters.client_id;
    let currency_label = filters.currency_label.as_deref();

    let total_invoiced_minor: i64 = connection.query_row(
        r#"
      SELECT COALESCE(SUM(i.total_minor), 0)
      FROM invoices i
      WHERE i.status != 'Draft'
        AND i.status != 'Cancelled'
        AND (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR i.currency_label = :currency_label)
        AND (:from_date IS NULL OR i.issue_date >= :from_date)
        AND (:to_date IS NULL OR i.issue_date <= :to_date)
    "#,
        named_params! {
          ":client_id": client_id,
          ":currency_label": currency_label,
          ":from_date": from_date,
          ":to_date": to_date,
        },
        |row| row.get(0),
    )?;

    let total_paid_minor: i64 = connection.query_row(
        r#"
      SELECT COALESCE(SUM(p.amount_minor), 0)
      FROM payments p
      JOIN invoices i ON i.id = p.invoice_id
      WHERE i.status != 'Draft'
        AND i.status != 'Cancelled'
        AND (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR p.currency_label = :currency_label)
        AND (:from_date IS NULL OR p.payment_date >= :from_date)
        AND (:to_date IS NULL OR p.payment_date <= :to_date)
    "#,
        named_params! {
          ":client_id": client_id,
          ":currency_label": currency_label,
          ":from_date": from_date,
          ":to_date": to_date,
        },
        |row| row.get(0),
    )?;

    let outstanding_balance_minor: i64 = connection.query_row(
        r#"
      SELECT COALESCE(SUM(i.outstanding_minor), 0)
      FROM invoices i
      WHERE i.status != 'Draft'
        AND i.status != 'Cancelled'
        AND (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR i.currency_label = :currency_label)
        AND (:from_date IS NULL OR i.issue_date >= :from_date)
        AND (:to_date IS NULL OR i.issue_date <= :to_date)
    "#,
        named_params! {
          ":client_id": client_id,
          ":currency_label": currency_label,
          ":from_date": from_date,
          ":to_date": to_date,
        },
        |row| row.get(0),
    )?;

    let invoice_count: i64 = connection.query_row(
        r#"
      SELECT COUNT(*)
      FROM invoices i
      WHERE (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR i.currency_label = :currency_label)
        AND (:from_date IS NULL OR i.issue_date >= :from_date)
        AND (:to_date IS NULL OR i.issue_date <= :to_date)
    "#,
        named_params! {
          ":client_id": client_id,
          ":currency_label": currency_label,
          ":from_date": from_date,
          ":to_date": to_date,
        },
        |row| row.get(0),
    )?;

    let overdue_invoice_count: i64 = connection.query_row(
        r#"
      SELECT COUNT(*)
      FROM invoices i
      WHERE i.status = 'Overdue'
        AND (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR i.currency_label = :currency_label)
        AND (:from_date IS NULL OR i.issue_date >= :from_date)
        AND (:to_date IS NULL OR i.issue_date <= :to_date)
    "#,
        named_params! {
          ":client_id": client_id,
          ":currency_label": currency_label,
          ":from_date": from_date,
          ":to_date": to_date,
        },
        |row| row.get(0),
    )?;

    let total_invoiced_by_currency =
        aggregate_invoices_by_currency(&connection, "SUM(total_minor)", &filters)?;
    let total_paid_by_currency = aggregate_payments_by_currency(&connection, &filters)?;
    let converted_income_by_currency = aggregate_conversions_by_currency(&connection, &filters)?;
    let reported_income_minor: i64 = connection.query_row(
        r#"
      SELECT COALESCE(SUM(cc.converted_amount_minor), 0)
      FROM currency_conversions cc
      JOIN payments p ON p.id = cc.payment_id
      JOIN invoices i ON i.id = p.invoice_id
      WHERE cc.target_currency_label = :reporting_currency_label
        AND (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR p.currency_label = :currency_label)
        AND (:from_date IS NULL OR p.payment_date >= :from_date)
        AND (:to_date IS NULL OR p.payment_date <= :to_date)
    "#,
        named_params! {
          ":reporting_currency_label": reporting_currency.as_str(),
          ":client_id": client_id,
          ":currency_label": currency_label,
          ":from_date": from_date,
          ":to_date": to_date,
        },
        |row| row.get(0),
    )?;

    let recent_invoices = query_recent_invoices(&connection, &filters, 5)?;
    let overdue_invoices = query_overdue_invoices(&connection, &filters, 5)?;
    let recent_payments = query_recent_payments(&connection, &filters, 5)?;

    Ok(DashboardSummary {
        total_invoiced_minor,
        total_paid_minor,
        outstanding_balance_minor,
        invoice_count,
        overdue_invoice_count,
        reporting_currency_label: reporting_currency,
        reported_income_minor,
        total_invoiced_by_currency,
        total_paid_by_currency,
        converted_income_by_currency,
        recent_invoices,
        overdue_invoices,
        recent_payments,
    })
}

fn aggregate_invoices_by_currency(
    connection: &rusqlite::Connection,
    amount_expression: &str,
    filters: &DashboardFilters,
) -> AppResult<Vec<CurrencyAggregate>> {
    let sql = format!(
        r#"
      SELECT i.currency_label AS currency_label,
             COALESCE({amount_expression}, 0) AS amount_minor,
             COUNT(*) AS count
      FROM invoices i
      WHERE i.status != 'Draft'
        AND i.status != 'Cancelled'
        AND (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR i.currency_label = :currency_label)
        AND (:from_date IS NULL OR i.issue_date >= :from_date)
        AND (:to_date IS NULL OR i.issue_date <= :to_date)
      GROUP BY i.currency_label
      ORDER BY i.currency_label ASC
    "#
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! {
          ":client_id": filters.client_id,
          ":currency_label": filters.currency_label.as_deref(),
          ":from_date": filters.from_date.as_deref(),
          ":to_date": filters.to_date.as_deref(),
        },
        |row| {
            Ok(CurrencyAggregate {
                currency_label: row.get("currency_label")?,
                amount_minor: row.get("amount_minor")?,
                count: row.get("count")?,
            })
        },
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn aggregate_payments_by_currency(
    connection: &rusqlite::Connection,
    filters: &DashboardFilters,
) -> AppResult<Vec<CurrencyAggregate>> {
    let mut statement = connection.prepare(
        r#"
      SELECT p.currency_label AS currency_label,
             COALESCE(SUM(p.amount_minor), 0) AS amount_minor,
             COUNT(*) AS count
      FROM payments p
      JOIN invoices i ON i.id = p.invoice_id
      WHERE i.status != 'Draft'
        AND i.status != 'Cancelled'
        AND (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR p.currency_label = :currency_label)
        AND (:from_date IS NULL OR p.payment_date >= :from_date)
        AND (:to_date IS NULL OR p.payment_date <= :to_date)
      GROUP BY p.currency_label
      ORDER BY p.currency_label ASC
    "#,
    )?;
    let rows = statement.query_map(
        named_params! {
          ":client_id": filters.client_id,
          ":currency_label": filters.currency_label.as_deref(),
          ":from_date": filters.from_date.as_deref(),
          ":to_date": filters.to_date.as_deref(),
        },
        |row| {
            Ok(CurrencyAggregate {
                currency_label: row.get("currency_label")?,
                amount_minor: row.get("amount_minor")?,
                count: row.get("count")?,
            })
        },
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn aggregate_conversions_by_currency(
    connection: &rusqlite::Connection,
    filters: &DashboardFilters,
) -> AppResult<Vec<CurrencyAggregate>> {
    let mut statement = connection.prepare(
        r#"
      SELECT cc.target_currency_label AS currency_label,
             COALESCE(SUM(cc.converted_amount_minor), 0) AS amount_minor,
             COUNT(*) AS count
      FROM currency_conversions cc
      JOIN payments p ON p.id = cc.payment_id
      JOIN invoices i ON i.id = p.invoice_id
      WHERE (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR p.currency_label = :currency_label)
        AND (:from_date IS NULL OR p.payment_date >= :from_date)
        AND (:to_date IS NULL OR p.payment_date <= :to_date)
      GROUP BY cc.target_currency_label
      ORDER BY cc.target_currency_label ASC
    "#,
    )?;
    let rows = statement.query_map(
        named_params! {
          ":client_id": filters.client_id,
          ":currency_label": filters.currency_label.as_deref(),
          ":from_date": filters.from_date.as_deref(),
          ":to_date": filters.to_date.as_deref(),
        },
        |row| {
            Ok(CurrencyAggregate {
                currency_label: row.get("currency_label")?,
                amount_minor: row.get("amount_minor")?,
                count: row.get("count")?,
            })
        },
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn query_recent_invoices(
    connection: &rusqlite::Connection,
    filters: &DashboardFilters,
    limit: i64,
) -> AppResult<Vec<InvoiceSummary>> {
    let mut statement = connection.prepare(
        r#"
      SELECT i.id, i.invoice_number, i.client_id, c.company_name AS client_company_name,
             i.status, i.issue_date, i.due_date, i.currency_label,
             i.total_minor, i.paid_minor, i.outstanding_minor,
             i.created_at, i.updated_at
      FROM invoices i
      JOIN clients c ON c.id = i.client_id
      WHERE (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR i.currency_label = :currency_label)
        AND (:from_date IS NULL OR i.issue_date >= :from_date)
        AND (:to_date IS NULL OR i.issue_date <= :to_date)
      ORDER BY i.created_at DESC, i.id DESC
      LIMIT :limit
    "#,
    )?;
    let rows = statement.query_map(
        named_params! {
            ":client_id": filters.client_id,
            ":currency_label": filters.currency_label.as_deref(),
            ":from_date": filters.from_date.as_deref(),
            ":to_date": filters.to_date.as_deref(),
            ":limit": limit,
        },
        row_to_invoice_summary,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn query_overdue_invoices(
    connection: &rusqlite::Connection,
    filters: &DashboardFilters,
    limit: i64,
) -> AppResult<Vec<InvoiceSummary>> {
    let mut statement = connection.prepare(
        r#"
      SELECT i.id, i.invoice_number, i.client_id, c.company_name AS client_company_name,
             i.status, i.issue_date, i.due_date, i.currency_label,
             i.total_minor, i.paid_minor, i.outstanding_minor,
             i.created_at, i.updated_at
      FROM invoices i
      JOIN clients c ON c.id = i.client_id
      WHERE i.status = 'Overdue'
        AND (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR i.currency_label = :currency_label)
        AND (:from_date IS NULL OR i.issue_date >= :from_date)
        AND (:to_date IS NULL OR i.issue_date <= :to_date)
      ORDER BY i.due_date ASC, i.created_at ASC, i.id ASC
      LIMIT :limit
    "#,
    )?;
    let rows = statement.query_map(
        named_params! {
            ":client_id": filters.client_id,
            ":currency_label": filters.currency_label.as_deref(),
            ":from_date": filters.from_date.as_deref(),
            ":to_date": filters.to_date.as_deref(),
            ":limit": limit,
        },
        row_to_invoice_summary,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn query_recent_payments(
    connection: &rusqlite::Connection,
    filters: &DashboardFilters,
    limit: i64,
) -> AppResult<Vec<Payment>> {
    let mut statement = connection.prepare(
        r#"
      SELECT p.*, i.invoice_number, c.company_name AS client_company_name,
             cc.target_currency_label AS reporting_currency_label,
             cc.conversion_rate AS conversion_rate
      FROM payments p
      JOIN invoices i ON i.id = p.invoice_id
      JOIN clients c ON c.id = i.client_id
      LEFT JOIN currency_conversions cc ON cc.payment_id = p.id
      WHERE (:client_id IS NULL OR i.client_id = :client_id)
        AND (:currency_label IS NULL OR p.currency_label = :currency_label)
        AND (:from_date IS NULL OR p.payment_date >= :from_date)
        AND (:to_date IS NULL OR p.payment_date <= :to_date)
      ORDER BY p.payment_date DESC, p.created_at DESC, p.id DESC
      LIMIT :limit
    "#,
    )?;
    let rows = statement.query_map(
        named_params! {
            ":client_id": filters.client_id,
            ":currency_label": filters.currency_label.as_deref(),
            ":from_date": filters.from_date.as_deref(),
            ":to_date": filters.to_date.as_deref(),
            ":limit": limit,
        },
        row_to_payment,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn export_invoices_csv(
    database: &Database,
    path: &Path,
    filters: Option<DashboardFilters>,
) -> AppResult<()> {
    let filter_values = filters.unwrap_or(DashboardFilters {
        from_date: None,
        to_date: None,
        client_id: None,
        currency_label: None,
    });
    let invoices = list_invoices(
        database,
        InvoiceFilters {
            search: None,
            client_id: filter_values.client_id,
            currency_label: filter_values.currency_label.clone(),
            status: None,
            from_date: filter_values.from_date.clone(),
            to_date: filter_values.to_date.clone(),
        },
    )?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record([
        "invoice_number",
        "client_company_name",
        "status",
        "issue_date",
        "due_date",
        "currency_label",
        "total_minor",
        "paid_minor",
        "outstanding_minor",
    ])?;
    for invoice in invoices {
        writer.write_record([
            invoice.invoice_number,
            invoice.client_company_name,
            invoice.status.as_str().to_string(),
            invoice.issue_date,
            invoice.due_date,
            invoice.currency_label,
            invoice.total_minor.to_string(),
            invoice.paid_minor.to_string(),
            invoice.outstanding_minor.to_string(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

pub fn export_payments_csv(
    database: &Database,
    path: &Path,
    filters: Option<DashboardFilters>,
) -> AppResult<()> {
    let filter_values = filters.unwrap_or(DashboardFilters {
        from_date: None,
        to_date: None,
        client_id: None,
        currency_label: None,
    });
    let payments = list_payments(
        database,
        PaymentFilters {
            search: None,
            invoice_id: None,
            client_id: filter_values.client_id,
            currency_label: filter_values.currency_label.clone(),
            from_date: filter_values.from_date.clone(),
            to_date: filter_values.to_date.clone(),
        },
    )?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record([
        "invoice_number",
        "client_company_name",
        "amount_minor",
        "currency_label",
        "converted_amount_minor",
        "reporting_currency_label",
        "conversion_rate",
        "payment_date",
        "payment_source",
        "transaction_reference_id",
        "notes",
    ])?;
    for payment in payments {
        writer.write_record([
            payment.invoice_number.unwrap_or_default(),
            payment.client_company_name.unwrap_or_default(),
            payment.amount_minor.to_string(),
            payment.currency_label,
            payment
                .converted_amount_minor
                .map(|value| value.to_string())
                .unwrap_or_default(),
            payment.reporting_currency_label,
            payment.conversion_rate.unwrap_or_default(),
            payment.payment_date,
            payment.payment_source.as_str().to_string(),
            payment.transaction_reference_id.unwrap_or_default(),
            payment.notes.unwrap_or_default(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

fn database_file_bytes(database: &Database) -> AppResult<Vec<u8>> {
    let bytes = fs::read(database.path())?;
    Ok(bytes)
}

pub fn export_database_backup(
    database: &Database,
    destination_dir: &Path,
    zip_archive: bool,
) -> AppResult<PathBuf> {
    fs::create_dir_all(destination_dir)?;
    database.checkpoint()?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let file_stem = format!("damn-simple-invoicing-backup-{timestamp}");

    if zip_archive {
        let output_path = destination_dir.join(format!("{file_stem}.zip"));
        let file = File::create(&output_path)?;
        let mut zip_writer = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip_writer.start_file("database.sqlite3", options)?;
        zip_writer.write_all(&database_file_bytes(database)?)?;
        zip_writer.start_file("manifest.json", options)?;
        let manifest = json!({
          "product": "Damn Simple Invoicing",
          "created_at": utc_now(),
          "database_path": database.path().to_string_lossy(),
        });
        zip_writer.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
        zip_writer.finish()?;
        Ok(output_path)
    } else {
        let output_path = destination_dir.join(format!("{file_stem}.sqlite3"));
        fs::copy(database.path(), &output_path)?;
        Ok(output_path)
    }
}

pub fn restore_database_backup(database: &Database, source_path: &Path) -> AppResult<()> {
    if !source_path.exists() {
        return Err(AppError::NotFound(format!(
            "backup file does not exist: {}",
            source_path.display()
        )));
    }

    if source_path.extension().and_then(|value| value.to_str()) == Some("zip") {
        let file = File::open(source_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|error| AppError::Backup(format!("unable to open zip archive: {error}")))?;
        let mut extracted = archive.by_name("database.sqlite3").map_err(|error| {
            AppError::Backup(format!(
                "backup archive is missing database.sqlite3: {error}"
            ))
        })?;
        let mut bytes = Vec::new();
        extracted.read_to_end(&mut bytes)?;
        fs::write(database.path(), bytes)?;
    } else {
        fs::copy(source_path, database.path())?;
    }

    database.migrate()?;
    Ok(())
}

fn open_path_with_os_default(path: &Path) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path.display().to_string()])
            .spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn()?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(AppError::Pdf(
        "opening files is not supported on this platform".to_string(),
    ))
}

pub fn open_local_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "path does not exist: {}",
            path.display()
        )));
    }

    open_path_with_os_default(path)
}

fn write_invoice_html(
    detail: &InvoiceDetail,
    output_dir: &Path,
) -> AppResult<(PathBuf, PathBuf, String)> {
    fs::create_dir_all(output_dir)?;
    let html_path = output_dir.join(format!("invoice-{}.html", detail.invoice.invoice_number));
    let pdf_path = output_dir.join(format!("invoice-{}.pdf", detail.invoice.invoice_number));
    let logo_data_uri = render_logo_data_uri(&detail.invoice.business_snapshot.logo_path);

    let line_items_rows = detail
    .line_items
    .iter()
    .map(|item| {
      format!(
        "<tr><td>{}</td><td class=\"text-right\">{}</td><td class=\"text-right\">{}</td><td class=\"text-right\">{}</td></tr>",
        escape_html(&item.description),
        escape_html(&item.quantity),
        money_display(item.rate_minor),
        money_display(item.line_total_minor)
      )
    })
    .collect::<String>();

    let payments_rows = detail
        .payments
        .iter()
        .map(|payment| {
            format!(
                "<tr><td>{}</td><td>{}</td><td class=\"text-right\">{} {}</td><td class=\"text-right\">{} {}</td><td>{}</td></tr>",
                escape_html(&payment.payment_date),
                escape_html(payment.payment_source.as_str()),
                money_display(payment.amount_minor),
                escape_html(&payment.currency_label),
                money_display(payment.converted_amount_minor.unwrap_or(payment.amount_minor)),
                escape_html(&payment.reporting_currency_label),
                escape_html(payment.transaction_reference_id.as_deref().unwrap_or("-"))
            )
        })
        .collect::<String>();

    let conversions_rows = detail
        .conversions
        .iter()
        .map(|conversion| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td class=\"text-right\">{}</td></tr>",
                escape_html(&conversion.source_currency_label),
                escape_html(&conversion.target_currency_label),
                escape_html(&conversion.conversion_rate),
                money_display(conversion.converted_amount_minor)
            )
        })
        .collect::<String>();

    let payments_section =
    if detail.payments.is_empty() {
        String::new()
    } else {
        format!(
            r#"
      <section>
        <h2 class="section-title">Payments</h2>
        <table>
          <thead>
            <tr>
              <th>Date</th>
              <th>Source</th>
              <th class="text-right">Paid</th>
              <th class="text-right">Reporting</th>
              <th>Reference</th>
            </tr>
          </thead>
          <tbody>
            {}
          </tbody>
        </table>
      </section>
"#,
            payments_rows
        )
    };

    let conversions_section =
    if detail.conversions.is_empty() {
        String::new()
    } else {
        format!(
            r#"
      <section>
        <h2 class="section-title">
          Conversion History
        </h2>

        <table>
          <thead>
            <tr>
              <th>Source</th>
              <th>Target</th>
              <th>Rate</th>
              <th class="text-right">
                Converted
              </th>
            </tr>
          </thead>

          <tbody>
            {}
          </tbody>
        </table>
      </section>
"#,
            conversions_rows
        )
    };

    let notes_value =
    detail.invoice.notes
        .as_deref()
        .unwrap_or("")
        .trim();

let payment_terms_value =
    detail.invoice.payment_terms
        .as_deref()
        .unwrap_or("")
        .trim();

let has_notes =
    !notes_value.is_empty();

let has_payment_terms =
    !payment_terms_value.is_empty();

let notes_section =
    if has_notes || has_payment_terms {
        format!(
            r#"
      <section class="card notes-card">
        <h2>Notes</h2>

        {}

        {}
      </section>
"#,
            if has_notes {
                format!(
                    "<p>{}</p>",
                    escape_html(notes_value)
                )
            } else {
                String::new()
            },
            if has_payment_terms {
                format!(
                    "<p><strong>Payment terms:</strong> {}</p>",
                    escape_html(payment_terms_value)
                )
            } else {
                String::new()
            }
        )
    } else {
        String::new()
    };

    let html = format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Invoice {invoice_number}</title>
    <style>
      :root {{
  color-scheme: light;

  --ink: #0f172a;
  --muted: #64748b;
  --line: #dbe3ef;
  --accent: #0891b2;
  --paper: #ffffff;
  --wash: #f8fafc;
}}

* {{
  box-sizing: border-box;
}}

html {{
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}}

body {{
  margin: 0;
  padding: 32px;
  font-family:
    "Inter",
    "Avenir Next",
    "Segoe UI",
    sans-serif;

  font-size: 14px;
  line-height: 1.45;

  color: var(--ink);

  background:
    linear-gradient(
      180deg,
      #f8fafc 0%,
      #edf3f8 100%
    );
}}

.sheet {{
  max-width: 960px;

  margin: 0 auto;

  background: var(--paper);

  border: 1px solid var(--line);

  border-radius: 24px;

  padding: 32px;

  box-shadow:
    0 24px 48px rgba(15, 23, 42, 0.08);
}}

.top {{
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 24px;
}}

.brand {{
  display: flex;
  gap: 16px;
  align-items: center;
}}

.brand img {{
  width: 56px;
  height: 56px;
  object-fit: cover;
  border-radius: 14px;
  flex-shrink: 0;
}}

.eyebrow {{
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--muted);
  font-size: 10px;
  font-weight: 700;
  margin-bottom: 6px;
}}

h1 {{
  font-size: 22px;
  line-height: 1.05;
  margin: 0;
}}

.muted {{
  color: var(--muted);
  margin-top: 6px;
  font-size: 13px;
}}

.meta-grid {{
  display: grid;
  grid-template-columns:
    repeat(2, minmax(0, 1fr));
  gap: 16px;
  margin: 18px 0 24px;
}}

.card {{
  border: 1px solid var(--line);
  border-radius: 18px;
  padding: 18px;
  background: var(--wash);
  page-break-inside: avoid;
}}

.card h2 {{
  margin: 0 0 12px;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  color: var(--muted);
  font-weight: 700;
}}

.stack p {{
  margin: 0 0 6px;
}}

.section-title {{
  margin: 28px 0 10px;
  font-size: 16px;
  font-weight: 700;
}}

table {{
  width: 100%;
  border-collapse: collapse;
  margin-top: 12px;
  page-break-inside: auto;
}}

thead {{
  display: table-header-group;
}}

tbody {{
  page-break-inside: auto;
}}

tr {{
  page-break-inside: avoid;
  page-break-after: auto;
}}

th,
td {{
  border-bottom: 1px solid var(--line);
  padding: 12px 10px;
  text-align: left;
  vertical-align: top;
}}

th {{
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.12em;
  font-size: 11px;
  font-weight: 700;
}}

td {{
  font-size: 13px;
}}

.text-right {{
  text-align: right;
}}

.summary {{
  margin-top: 24px;
  display: grid;
  grid-template-columns:
    repeat(4, minmax(0, 1fr));
  gap: 12px;
  page-break-inside: avoid;
}}

.summary .card {{
  background: white;
}}

.summary .card strong {{
  display: block;
  font-size: 12px;
  line-height: 1.1;
  margin-top: 10px;
}}

.notes-card {{
  margin-top: 24px;
}}

@media print {{
  html,
  body {{
    background: white !important;
  }}

  body {{
    padding: 0 !important;
    font-size: 12px;
  }}

  .sheet {{
    max-width: 100% !important;
    margin: 0 !important;
    border: none !important;
    border-radius: 0 !important;
    box-shadow: none !important;
    padding: 0 !important;
  }}

  .card {{
    background: #f8fafc !important;
  }}

  section,
  article,
  table,
  tr,
  td,
  th,
  .card,
  .summary {{
    page-break-inside: avoid;
  }}

  .summary {{
    margin-top: 18px;
  }}

  @page {{
    size: A4;
    margin: 10mm;
  }}
}}
    </style>
  </head>
  <body>
    <main class="sheet">
      <section class="top">
        <div class="brand">
          {logo_html}
          <div>
            <div class="eyebrow">Invoice</div>
            <h1>{invoice_number}</h1>
            <div class="muted">{status}</div>
          </div>
        </div>
        <div class="stack" style="text-align:right">
          <p><strong>Issue date:</strong> {issue_date}</p>
          <p><strong>Due date:</strong> {due_date}</p>
          <p><strong>Currency:</strong> {currency_label}</p>
        </div>
      </section>

      <section class="meta-grid">
        <article class="card">
          <h2>Business</h2>
          <div class="stack">
            <p><strong>{business_name}</strong></p>
            <p>{business_legal_name}</p>
            <p>{business_address}</p>
            <p>{business_country}</p>
            <p>{business_email}</p>
            <p>{business_phone}</p>
            <p>{business_registration}</p>
            <p>{business_tax}</p>
          </div>
        </article>
        <article class="card">
          <h2>Client</h2>
          <div class="stack">
            <p><strong>{client_name}</strong></p>
            <p>{client_contact}</p>
            <p>{client_address}</p>
            <p>{client_country}</p>
            <p>{client_email}</p>
            <p>{client_notes}</p>
          </div>
        </article>
      </section>

      <section>
        <h2 class="section-title">Line Items</h2>
        <table>
          <thead>
            <tr>
              <th>Description</th>
              <th class="text-right">Quantity</th>
              <th class="text-right">Rate</th>
              <th class="text-right">Total</th>
            </tr>
          </thead>
          <tbody>
            {line_items_rows}
          </tbody>
        </table>
      </section>

      <section class="summary">
        <article class="card"><span class="muted">Subtotal</span><strong>{subtotal}</strong></article>
        <article class="card"><span class="muted">Paid</span><strong>{paid}</strong></article>
        <article class="card"><span class="muted">Outstanding</span><strong>{outstanding}</strong></article>
        <article class="card"><span class="muted">Total</span><strong>{total}</strong></article>
      </section>

      {payments_section}
      {conversions_section}
      {notes_section}
    </main>
  </body>
</html>"#,
        invoice_number = escape_html(&detail.invoice.invoice_number),
        status = escape_html(detail.invoice.status.as_str()),
        issue_date = escape_html(&detail.invoice.issue_date),
        due_date = escape_html(&detail.invoice.due_date),
        currency_label = escape_html(&detail.invoice.currency_label),
        business_name = escape_html(&detail.invoice.business_snapshot.business_name),
        business_legal_name = escape_html(
            detail
                .invoice
                .business_snapshot
                .legal_name
                .as_deref()
                .unwrap_or("-")
        ),
        business_address = escape_html(
            detail
                .invoice
                .business_snapshot
                .address
                .as_deref()
                .unwrap_or("-")
        ),
        business_country = escape_html(
            detail
                .invoice
                .business_snapshot
                .country
                .as_deref()
                .unwrap_or("-")
        ),
        business_email = escape_html(
            detail
                .invoice
                .business_snapshot
                .email
                .as_deref()
                .unwrap_or("-")
        ),
        business_phone = escape_html(
            detail
                .invoice
                .business_snapshot
                .phone
                .as_deref()
                .unwrap_or("-")
        ),
        business_registration = escape_html(
            detail
                .invoice
                .business_snapshot
                .registration_number
                .as_deref()
                .unwrap_or("-")
        ),
        business_tax = escape_html(
            detail
                .invoice
                .business_snapshot
                .tax_vat_number
                .as_deref()
                .unwrap_or("-")
        ),
        client_name = escape_html(&detail.invoice.client_snapshot.company_name),
        client_contact = escape_html(
            detail
                .invoice
                .client_snapshot
                .contact_person
                .as_deref()
                .unwrap_or("-")
        ),
        client_address = escape_html(
            detail
                .invoice
                .client_snapshot
                .address
                .as_deref()
                .unwrap_or("-")
        ),
        client_country = escape_html(
            detail
                .invoice
                .client_snapshot
                .country
                .as_deref()
                .unwrap_or("-")
        ),
        client_email = escape_html(
            detail
                .invoice
                .client_snapshot
                .email
                .as_deref()
                .unwrap_or("-")
        ),
        client_notes = escape_html(
            detail
                .invoice
                .client_snapshot
                .notes
                .as_deref()
                .unwrap_or("-")
        ),
        subtotal = money_display(detail.invoice.subtotal_minor),
        paid = money_display(detail.invoice.paid_minor),
        outstanding = money_display(detail.invoice.outstanding_minor),
        total = money_display(detail.invoice.total_minor),
        logo_html = logo_data_uri
            .map(|uri| format!(r#"<img alt="logo" src="{uri}" />"#))
            .unwrap_or_else(String::new),
        payments_section = payments_section,
        conversions_section = conversions_section,
        notes_section = notes_section,
    );

    fs::write(&html_path, html.as_bytes())?;
    Ok((html_path, pdf_path, html))
}

pub fn generate_invoice_preview(
    database: &Database,
    invoice_id: i64,
    output_dir: Option<PathBuf>,
) -> AppResult<PdfExportResult> {
    let detail = get_invoice(database, invoice_id)?;

    let output_dir = output_dir.unwrap_or_else(|| {
        database
            .path()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("exports")
    });

    let (html_path, pdf_path, _) = write_invoice_html(&detail, &output_dir)?;

    Ok(PdfExportResult {
        html_path: html_path.to_string_lossy().to_string(),

        pdf_path: pdf_path.to_string_lossy().to_string(),
    })
}

pub fn open_invoice_pdf(
    database: &Database,
    invoice_id: i64,
    output_dir: Option<PathBuf>,
) -> AppResult<PdfExportResult> {
    let result = generate_invoice_preview(database, invoice_id, output_dir)?;
    Ok(result)
}

pub fn export_invoice_html_only(
    database: &Database,
    invoice_id: i64,
    output_dir: Option<PathBuf>,
) -> AppResult<PdfExportResult> {
    let detail = get_invoice(database, invoice_id)?;
    let output_dir = output_dir.unwrap_or_else(|| {
        database
            .path()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("exports")
    });
    let (html_path, pdf_path, _) = write_invoice_html(&detail, &output_dir)?;
    Ok(PdfExportResult {
        html_path: html_path.to_string_lossy().to_string(),
        pdf_path: pdf_path.to_string_lossy().to_string(),
    })
}
