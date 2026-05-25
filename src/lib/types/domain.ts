export type InvoiceStatus =
  | 'Draft'
  | 'Sent'
  | 'Partially Paid'
  | 'Paid'
  | 'Overdue'
  | 'Cancelled';

export type PaymentSource = 'Wise Business' | 'Bank Transfer' | 'PayPal' | 'Other';

export interface Money {
  amountMinor: number;
  currency: string;
}

export interface BusinessProfile {
  id: number;
  businessName: string;
  legalName: string | null;
  address: string | null;
  country: string | null;
  email: string | null;
  phone: string | null;
  registrationNumber: string | null;
  taxVatNumber: string | null;
  logoPath: string | null;
  isActive: boolean;
  archivedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface BusinessProfileInput {
  businessName: string;
  legalName?: string | null;
  address?: string | null;
  country?: string | null;
  email?: string | null;
  phone?: string | null;
  registrationNumber?: string | null;
  taxVatNumber?: string | null;
  logoPath?: string | null;
  isActive: boolean;
}

export interface BusinessSnapshot {
  businessId: number;
  businessName: string;
  legalName: string | null;
  address: string | null;
  country: string | null;
  email: string | null;
  phone: string | null;
  registrationNumber: string | null;
  taxVatNumber: string | null;
  logoPath: string | null;
  capturedAt: string;
}

export interface Client {
  id: number;
  companyName: string;
  contactPerson: string | null;
  email: string | null;
  address: string | null;
  country: string | null;
  notes: string | null;
  archivedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ClientInput {
  companyName: string;
  contactPerson?: string | null;
  email?: string | null;
  address?: string | null;
  country?: string | null;
  notes?: string | null;
}

export interface ClientSnapshot {
  clientId: number;
  companyName: string;
  contactPerson: string | null;
  email: string | null;
  address: string | null;
  country: string | null;
  notes: string | null;
  capturedAt: string;
}

export interface InvoiceLineItem {
  id: number;
  invoiceId: number;
  position: number;
  description: string;
  quantity: string;
  rateMinor: number;
  lineTotalMinor: number;
  createdAt: string;
  updatedAt: string;
}

export interface InvoiceLineItemInput {
  description: string;
  quantity: string;
  rateMinor: number;
}

export interface InvoiceTotalsSnapshot {
  subtotalMinor: number;
  totalMinor: number;
  paidMinor: number;
  outstandingMinor: number;
  currencyLabel: string;
  capturedAt: string;
}

export interface CurrencyConversion {
  id: number;
  invoiceId: number | null;
  paymentId: number | null;
  sourceCurrencyLabel: string;
  targetCurrencyLabel: string;
  conversionRate: string;
  sourceAmountMinor: number;
  convertedAmountMinor: number;
  capturedAt: string;
}

export interface InvoiceRecord {
  id: number;
  invoiceNumber: string;
  businessId: number;
  clientId: number;
  status: InvoiceStatus;
  issueDate: string;
  dueDate: string;
  currencyLabel: string;
  notes: string | null;
  paymentTerms: string | null;
  subtotalMinor: number;
  totalMinor: number;
  paidMinor: number;
  outstandingMinor: number;
  finalizedAt: string | null;
  lockedAt: string | null;
  cancelledAt: string | null;
  businessSnapshot: BusinessSnapshot;
  clientSnapshot: ClientSnapshot;
  totalsSnapshot: InvoiceTotalsSnapshot;
  conversionSnapshot: CurrencyConversion[];
  createdAt: string;
  updatedAt: string;
}

export interface InvoiceSummary {
  id: number;
  invoiceNumber: string;
  clientId: number;
  clientCompanyName: string;
  status: InvoiceStatus;
  issueDate: string;
  dueDate: string;
  currencyLabel: string;
  totalMinor: number;
  paidMinor: number;
  outstandingMinor: number;
  createdAt: string;
  updatedAt: string;
}

export interface InvoiceDetail {
  invoice: InvoiceRecord;
  lineItems: InvoiceLineItem[];
  payments: Payment[];
  conversions: CurrencyConversion[];
}

export interface InvoiceFormInput {
  businessId?: number | null;
  clientId: number;
  invoiceNumber?: string | null;
  issueDate: string;
  dueDate: string;
  currencyLabel: string;
  notes?: string | null;
  paymentTerms?: string | null;
  lineItems: InvoiceLineItemInput[];
}

export interface InvoiceFilters {
  search?: string | null;
  clientId?: number | null;
  currencyLabel?: string | null;
  status?: InvoiceStatus | null;
  fromDate?: string | null;
  toDate?: string | null;
}

export interface Payment {
  id: number;
  invoiceId: number;
  invoiceNumber: string | null;
  clientCompanyName: string | null;
  amountMinor: number;
  currencyLabel: string;
  convertedAmountMinor: number | null;
  reportingCurrencyLabel: string;
  conversionRate: string | null;
  paymentDate: string;
  paymentSource: PaymentSource;
  transactionReferenceId: string | null;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface PaymentInput {
  invoiceId: number;
  amountMinor: number;
  currencyLabel: string;
  reportingCurrencyLabel?: string | null;
  convertedAmountMinor?: number | null;
  conversionRate?: string | null;
  paymentDate: string;
  paymentSource: PaymentSource;
  transactionReferenceId?: string | null;
  notes?: string | null;
}

export interface PaymentFilters {
  search?: string | null;
  invoiceId?: number | null;
  clientId?: number | null;
  currencyLabel?: string | null;
  fromDate?: string | null;
  toDate?: string | null;
}

export interface CurrencyAggregate {
  currencyLabel: string;
  amountMinor: number;
  count: number;
}

export interface DashboardFilters {
  fromDate?: string | null;
  toDate?: string | null;
  clientId?: number | null;
  currencyLabel?: string | null;
}

export interface DashboardSummary {
  totalInvoicedMinor: number;
  totalPaidMinor: number;
  outstandingBalanceMinor: number;
  invoiceCount: number;
  overdueInvoiceCount: number;
  reportingCurrencyLabel: string;
  reportedIncomeMinor: number;
  totalInvoicedByCurrency: CurrencyAggregate[];
  totalPaidByCurrency: CurrencyAggregate[];
  convertedIncomeByCurrency: CurrencyAggregate[];
  recentInvoices: InvoiceSummary[];
  overdueInvoices: InvoiceSummary[];
  recentPayments: Payment[];
}

export interface AppBootState {
  databasePath: string;
  activeBusiness: BusinessProfile | null;
  reportingCurrencyLabel: string;
  settings: Record<string, string>;
}

export interface ClientDetail {
  client: Client;
  invoiceHistory: InvoiceSummary[];
}

export interface BackupExportInput {
  destinationDir: string;
  zip: boolean;
}

export interface BackupExportResult {
  path: string;
}

export interface PdfExportInput {
  invoiceId: number;
  outputDir?: string | null;
  openAfter: boolean;
}

export interface PdfExportResult {
  htmlPath: string;
  pdfPath: string;
}

export interface CsvExportInput {
  path: string;
  filters?: DashboardFilters | null;
}

export interface CsvExportResult {
  path: string;
}

export interface SettingPair {
  key: string;
  value: string;
}
