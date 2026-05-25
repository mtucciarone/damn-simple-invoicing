use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Money {
    pub amount_minor: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvoiceStatus {
    #[serde(rename = "Draft")]
    Draft,
    #[serde(rename = "Sent")]
    Sent,
    #[serde(rename = "Partially Paid")]
    PartiallyPaid,
    #[serde(rename = "Paid")]
    Paid,
    #[serde(rename = "Overdue")]
    Overdue,
    #[serde(rename = "Cancelled")]
    Cancelled,
}

impl InvoiceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "Draft",
            InvoiceStatus::Sent => "Sent",
            InvoiceStatus::PartiallyPaid => "Partially Paid",
            InvoiceStatus::Paid => "Paid",
            InvoiceStatus::Overdue => "Overdue",
            InvoiceStatus::Cancelled => "Cancelled",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "Draft" => Some(Self::Draft),
            "Sent" => Some(Self::Sent),
            "Partially Paid" => Some(Self::PartiallyPaid),
            "Paid" => Some(Self::Paid),
            "Overdue" => Some(Self::Overdue),
            "Cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentSource {
    #[serde(rename = "Wise Business")]
    WiseBusiness,
    #[serde(rename = "Bank Transfer")]
    BankTransfer,
    #[serde(rename = "PayPal")]
    PayPal,
    #[serde(rename = "Other")]
    Other,
}

impl PaymentSource {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentSource::WiseBusiness => "Wise Business",
            PaymentSource::BankTransfer => "Bank Transfer",
            PaymentSource::PayPal => "PayPal",
            PaymentSource::Other => "Other",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "Wise Business" => Some(Self::WiseBusiness),
            "Bank Transfer" => Some(Self::BankTransfer),
            "PayPal" => Some(Self::PayPal),
            "Other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessProfile {
    pub id: i64,
    pub business_name: String,
    pub legal_name: Option<String>,
    pub address: Option<String>,
    pub country: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub registration_number: Option<String>,
    pub tax_vat_number: Option<String>,
    pub logo_path: Option<String>,
    pub is_active: bool,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessProfileInput {
    pub business_name: String,
    pub legal_name: Option<String>,
    pub address: Option<String>,
    pub country: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub registration_number: Option<String>,
    pub tax_vat_number: Option<String>,
    pub logo_path: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessSnapshot {
    pub business_id: i64,
    pub business_name: String,
    pub legal_name: Option<String>,
    pub address: Option<String>,
    pub country: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub registration_number: Option<String>,
    pub tax_vat_number: Option<String>,
    pub logo_path: Option<String>,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id: i64,
    pub company_name: String,
    pub contact_person: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub country: Option<String>,
    pub notes: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInput {
    pub company_name: String,
    pub contact_person: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub country: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSnapshot {
    pub client_id: i64,
    pub company_name: String,
    pub contact_person: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub country: Option<String>,
    pub notes: Option<String>,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLineItem {
    pub id: i64,
    pub invoice_id: i64,
    pub position: i64,
    pub description: String,
    pub quantity: String,
    pub rate_minor: i64,
    pub line_total_minor: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLineItemInput {
    pub description: String,
    pub quantity: String,
    pub rate_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceTotalsSnapshot {
    pub subtotal_minor: i64,
    pub total_minor: i64,
    pub paid_minor: i64,
    pub outstanding_minor: i64,
    pub currency_label: String,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyConversion {
    pub id: i64,
    pub invoice_id: Option<i64>,
    pub payment_id: Option<i64>,
    pub source_currency_label: String,
    pub target_currency_label: String,
    pub conversion_rate: String,
    pub source_amount_minor: i64,
    pub converted_amount_minor: i64,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceRecord {
    pub id: i64,
    pub invoice_number: String,
    pub business_id: i64,
    pub client_id: i64,
    pub status: InvoiceStatus,
    pub issue_date: String,
    pub due_date: String,
    pub currency_label: String,
    pub notes: Option<String>,
    pub payment_terms: Option<String>,
    pub subtotal_minor: i64,
    pub total_minor: i64,
    pub paid_minor: i64,
    pub outstanding_minor: i64,
    pub finalized_at: Option<String>,
    pub locked_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub business_snapshot: BusinessSnapshot,
    pub client_snapshot: ClientSnapshot,
    pub totals_snapshot: InvoiceTotalsSnapshot,
    pub conversion_snapshot: Vec<CurrencyConversion>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceSummary {
    pub id: i64,
    pub invoice_number: String,
    pub client_id: i64,
    pub client_company_name: String,
    pub status: InvoiceStatus,
    pub issue_date: String,
    pub due_date: String,
    pub currency_label: String,
    pub total_minor: i64,
    pub paid_minor: i64,
    pub outstanding_minor: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceDetail {
    pub invoice: InvoiceRecord,
    pub line_items: Vec<InvoiceLineItem>,
    pub payments: Vec<Payment>,
    pub conversions: Vec<CurrencyConversion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceFormInput {
    pub business_id: Option<i64>,
    pub client_id: i64,
    pub invoice_number: Option<String>,
    pub issue_date: String,
    pub due_date: String,
    pub currency_label: String,
    pub notes: Option<String>,
    pub payment_terms: Option<String>,
    pub line_items: Vec<InvoiceLineItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceFilters {
    pub search: Option<String>,
    pub client_id: Option<i64>,
    pub currency_label: Option<String>,
    pub status: Option<InvoiceStatus>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub id: i64,
    pub invoice_id: i64,
    pub invoice_number: Option<String>,
    pub client_company_name: Option<String>,
    pub amount_minor: i64,
    pub currency_label: String,
    pub converted_amount_minor: Option<i64>,
    pub reporting_currency_label: String,
    pub conversion_rate: Option<String>,
    pub payment_date: String,
    pub payment_source: PaymentSource,
    pub transaction_reference_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInput {
    pub invoice_id: i64,
    pub amount_minor: i64,
    pub currency_label: String,
    pub reporting_currency_label: Option<String>,
    pub converted_amount_minor: Option<i64>,
    pub conversion_rate: Option<String>,
    pub payment_date: String,
    pub payment_source: PaymentSource,
    pub transaction_reference_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentFilters {
    pub search: Option<String>,
    pub invoice_id: Option<i64>,
    pub client_id: Option<i64>,
    pub currency_label: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyAggregate {
    pub currency_label: String,
    pub amount_minor: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardFilters {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub client_id: Option<i64>,
    pub currency_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub total_invoiced_minor: i64,
    pub total_paid_minor: i64,
    pub outstanding_balance_minor: i64,
    pub invoice_count: i64,
    pub overdue_invoice_count: i64,
    pub reporting_currency_label: String,
    pub reported_income_minor: i64,
    pub total_invoiced_by_currency: Vec<CurrencyAggregate>,
    pub total_paid_by_currency: Vec<CurrencyAggregate>,
    pub converted_income_by_currency: Vec<CurrencyAggregate>,
    pub recent_invoices: Vec<InvoiceSummary>,
    pub overdue_invoices: Vec<InvoiceSummary>,
    pub recent_payments: Vec<Payment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootState {
    pub database_path: String,
    pub active_business: Option<BusinessProfile>,
    pub reporting_currency_label: String,
    pub settings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientDetail {
    pub client: Client,
    pub invoice_history: Vec<InvoiceSummary>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupExportInput {
    pub destination_dir: String,
    pub zip: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupExportResult {
    pub path: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfExportInput {
    pub invoice_id: i64,
    pub output_dir: Option<String>,
    pub open_after: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfExportResult {
    pub html_path: String,
    pub pdf_path: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvExportInput {
    pub path: String,
    pub filters: Option<DashboardFilters>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvExportResult {
    pub path: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingPair {
    pub key: String,
    pub value: String,
}
