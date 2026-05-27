import { invoke } from '@tauri-apps/api/core';

import type {
  AppBootState,
  BackupExportResult,
  BusinessProfile,
  BusinessProfileInput,
  Client,
  ClientDetail,
  ClientInput,
  CsvExportResult,
  DashboardFilters,
  DashboardSummary,
  InvoiceDetail,
  InvoiceFilters,
  InvoiceFormInput,
  InvoiceSummary,
  Payment,
  PaymentFilters,
  PaymentInput,
  PdfExportResult,
} from '$lib/types/domain';

export async function getAppState() {
  return invoke<AppBootState>('get_app_state');
}

export async function getAppSettings() {
  return invoke<Record<string, string>>('get_app_settings');
}

export async function updateAppSettings(settings: Record<string, string>) {
  return invoke<Record<string, string>>('update_app_settings', { settings });
}

export async function listBusinesses(includeArchived = false) {
  return invoke<BusinessProfile[]>('list_businesses', { includeArchived });
}

export async function getActiveBusiness() {
  return invoke<BusinessProfile | null>('get_active_business');
}

export async function getBusiness(id: number) {
  return invoke<BusinessProfile>('get_business', { id });
}

export async function createBusiness(input: BusinessProfileInput) {
  return invoke<BusinessProfile>('create_business', { input });
}

export async function updateBusiness(id: number, input: BusinessProfileInput) {
  return invoke<BusinessProfile>('update_business', { id, input });
}

export async function archiveBusiness(id: number) {
  return invoke<BusinessProfile>('archive_business', { id });
}

export async function setActiveBusiness(id: number) {
  return invoke<BusinessProfile>('set_active_business', { id });
}

export async function listClients(search: string | null = null, includeArchived = false) {
  return invoke<Client[]>('list_clients', { search, includeArchived });
}

export async function getClient(id: number) {
  return invoke<ClientDetail>('get_client', { id });
}

export async function createClient(input: ClientInput) {
  return invoke<Client>('create_client', { input });
}

export async function updateClient(id: number, input: ClientInput) {
  return invoke<Client>('update_client', { id, input });
}

export async function archiveClient(id: number) {
  return invoke<Client>('archive_client', { id });
}

export async function deleteClient(id: number) {
  return invoke<void>('delete_client', { id });
}

export async function createInvoice(input: InvoiceFormInput) {
  return invoke<InvoiceDetail>('create_invoice', { input });
}

export async function updateInvoice(id: number, input: InvoiceFormInput) {
  return invoke<InvoiceDetail>('update_invoice', { id, input });
}

export async function duplicateInvoice(id: number) {
  return invoke<InvoiceDetail>('duplicate_invoice', { id });
}

export async function deleteInvoice(id: number) {
  return invoke<void>('delete_invoice', { id });
}

export async function deleteDraftInvoice(id: number) {
  return deleteInvoice(id);
}

export async function finalizeInvoice(id: number) {
  return invoke<InvoiceDetail>('finalize_invoice', { id });
}

export async function reorderInvoiceLineItems(invoiceId: number, orderedLineItemIds: number[]) {
  return invoke<void>('reorder_invoice_line_items', {
    invoiceId,
    orderedLineItemIds,
  });
}

export async function getInvoice(invoiceId: number) {
  return invoke<InvoiceDetail>('get_invoice', { invoiceId });
}

export async function listInvoices(filters: InvoiceFilters = {}) {
  return invoke<InvoiceSummary[]>('list_invoices', { filters });
}

export async function recordPayment(input: PaymentInput) {
  return invoke<Payment>('record_payment', { input });
}

export async function updatePayment(paymentId: number, input: PaymentInput) {
  return invoke<Payment>('update_payment', { paymentId, input });
}

export async function deletePayment(paymentId: number) {
  return invoke<void>('delete_payment', { paymentId });
}

export async function getPayment(paymentId: number) {
  return invoke<Payment>('get_payment', { paymentId });
}

export async function listPayments(filters: PaymentFilters = {}) {
  return invoke<Payment[]>('list_payments', { filters });
}

export async function getDashboardSummary(filters: DashboardFilters = {}) {
  return invoke<DashboardSummary>('get_dashboard_summary', { filters });
}

export async function exportInvoicesCsv(path: string, filters: DashboardFilters | null = null) {
  return invoke<CsvExportResult>('export_invoices_csv', { path, filters });
}

export async function exportPaymentsCsv(path: string, filters: DashboardFilters | null = null) {
  return invoke<CsvExportResult>('export_payments_csv', { path, filters });
}

export async function exportDatabaseBackup(destinationDir: string, zip = false) {
  return invoke<BackupExportResult>('export_database_backup', {
    destinationDir,
    zip,
  });
}

export async function restoreDatabaseBackup(sourcePath: string) {
  return invoke<void>('restore_database_backup', { sourcePath });
}

export async function openLocalPath(path: string) {
  return invoke<void>('open_local_path', { path });
}

export async function exportInvoicePdf(
  invoiceId: number,
  outputDir: string | null = null,
) {
  return invoke<PdfExportResult>('generate_invoice_preview', {
    invoiceId,
    outputDir,
  });
}

export async function openInvoicePdf(invoiceId: number, outputDir: string | null = null) {
  return invoke<PdfExportResult>('open_invoice_pdf', {
    invoiceId,
    outputDir,
  });
}

export async function exportInvoiceHtmlOnly(invoiceId: number, outputDir: string | null = null) {
  return invoke<PdfExportResult>('export_invoice_html_only', {
    invoiceId,
    outputDir,
  });
}
