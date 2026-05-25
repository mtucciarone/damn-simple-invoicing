PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS businesses (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  is_active INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0, 1)),
  business_name TEXT NOT NULL,
  legal_name TEXT,
  address TEXT,
  country TEXT,
  email TEXT,
  phone TEXT,
  registration_number TEXT,
  tax_vat_number TEXT,
  logo_path TEXT,
  archived_at TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_businesses_one_active
  ON businesses(is_active)
  WHERE is_active = 1;

CREATE INDEX IF NOT EXISTS idx_businesses_archived_at ON businesses(archived_at);

CREATE TABLE IF NOT EXISTS clients (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_name TEXT NOT NULL,
  contact_person TEXT,
  email TEXT,
  address TEXT,
  country TEXT,
  notes TEXT,
  archived_at TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_clients_company_name ON clients(company_name);
CREATE INDEX IF NOT EXISTS idx_clients_archived_at ON clients(archived_at);

CREATE TABLE IF NOT EXISTS invoices (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  invoice_number TEXT NOT NULL UNIQUE,
  business_id INTEGER NOT NULL,
  client_id INTEGER NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('Draft', 'Sent', 'Partially Paid', 'Paid', 'Overdue', 'Cancelled')),
  issue_date TEXT NOT NULL,
  due_date TEXT NOT NULL,
  currency_label TEXT NOT NULL,
  notes TEXT,
  payment_terms TEXT,
  subtotal_minor INTEGER NOT NULL DEFAULT 0,
  total_minor INTEGER NOT NULL DEFAULT 0,
  paid_minor INTEGER NOT NULL DEFAULT 0,
  outstanding_minor INTEGER NOT NULL DEFAULT 0,
  finalized_at TEXT,
  locked_at TEXT,
  cancelled_at TEXT,
  business_snapshot_json TEXT NOT NULL,
  client_snapshot_json TEXT NOT NULL,
  totals_snapshot_json TEXT NOT NULL,
  conversion_snapshot_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  FOREIGN KEY (business_id) REFERENCES businesses(id) ON DELETE RESTRICT,
  FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_invoices_business_id ON invoices(business_id);
CREATE INDEX IF NOT EXISTS idx_invoices_client_id ON invoices(client_id);
CREATE INDEX IF NOT EXISTS idx_invoices_status ON invoices(status);
CREATE INDEX IF NOT EXISTS idx_invoices_issue_date ON invoices(issue_date);
CREATE INDEX IF NOT EXISTS idx_invoices_due_date ON invoices(due_date);
CREATE INDEX IF NOT EXISTS idx_invoices_currency_label ON invoices(currency_label);

CREATE TABLE IF NOT EXISTS invoice_line_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  invoice_id INTEGER NOT NULL,
  position INTEGER NOT NULL,
  description TEXT NOT NULL,
  quantity TEXT NOT NULL,
  rate_minor INTEGER NOT NULL,
  line_total_minor INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_invoice_line_items_invoice_position
  ON invoice_line_items(invoice_id, position);

CREATE INDEX IF NOT EXISTS idx_invoice_line_items_invoice_id ON invoice_line_items(invoice_id);

CREATE TABLE IF NOT EXISTS payments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  invoice_id INTEGER NOT NULL,
  amount_minor INTEGER NOT NULL,
  currency_label TEXT NOT NULL,
  converted_amount_minor INTEGER,
  payment_date TEXT NOT NULL,
  payment_source TEXT NOT NULL CHECK (payment_source IN ('Wise Business', 'Bank Transfer', 'PayPal', 'Other')),
  transaction_reference_id TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_payments_invoice_id ON payments(invoice_id);
CREATE INDEX IF NOT EXISTS idx_payments_payment_date ON payments(payment_date);
CREATE INDEX IF NOT EXISTS idx_payments_currency_label ON payments(currency_label);

CREATE TABLE IF NOT EXISTS currency_conversions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  invoice_id INTEGER,
  payment_id INTEGER,
  source_currency_label TEXT NOT NULL,
  target_currency_label TEXT NOT NULL,
  conversion_rate TEXT NOT NULL,
  source_amount_minor INTEGER NOT NULL,
  converted_amount_minor INTEGER NOT NULL,
  captured_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE,
  FOREIGN KEY (payment_id) REFERENCES payments(id) ON DELETE CASCADE,
  CHECK (invoice_id IS NOT NULL OR payment_id IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_currency_conversions_invoice_id ON currency_conversions(invoice_id);
CREATE INDEX IF NOT EXISTS idx_currency_conversions_payment_id ON currency_conversions(payment_id);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TRIGGER IF NOT EXISTS trg_invoice_line_items_insert_locked
BEFORE INSERT ON invoice_line_items
WHEN EXISTS (
  SELECT 1 FROM invoices
  WHERE invoices.id = NEW.invoice_id
    AND invoices.locked_at IS NOT NULL
)
BEGIN
  SELECT RAISE(ABORT, 'finalized invoices are immutable');
END;

CREATE TRIGGER IF NOT EXISTS trg_invoice_line_items_update_locked
BEFORE UPDATE ON invoice_line_items
WHEN EXISTS (
  SELECT 1 FROM invoices
  WHERE invoices.id = OLD.invoice_id
    AND invoices.locked_at IS NOT NULL
)
BEGIN
  SELECT RAISE(ABORT, 'finalized invoices are immutable');
END;

CREATE TRIGGER IF NOT EXISTS trg_invoice_line_items_delete_locked
BEFORE DELETE ON invoice_line_items
WHEN EXISTS (
  SELECT 1 FROM invoices
  WHERE invoices.id = OLD.invoice_id
    AND invoices.locked_at IS NOT NULL
)
BEGIN
  SELECT RAISE(ABORT, 'finalized invoices are immutable');
END;

CREATE TRIGGER IF NOT EXISTS trg_invoices_update_locked
BEFORE UPDATE ON invoices
WHEN OLD.locked_at IS NOT NULL AND (
  NEW.business_id != OLD.business_id OR
  NEW.client_id != OLD.client_id OR
  NEW.invoice_number != OLD.invoice_number OR
  NEW.issue_date != OLD.issue_date OR
  NEW.due_date != OLD.due_date OR
  NEW.currency_label != OLD.currency_label OR
  COALESCE(NEW.notes, '') != COALESCE(OLD.notes, '') OR
  COALESCE(NEW.payment_terms, '') != COALESCE(OLD.payment_terms, '') OR
  NEW.subtotal_minor != OLD.subtotal_minor OR
  NEW.total_minor != OLD.total_minor OR
  COALESCE(NEW.business_snapshot_json, '') != COALESCE(OLD.business_snapshot_json, '') OR
  COALESCE(NEW.client_snapshot_json, '') != COALESCE(OLD.client_snapshot_json, '') OR
  COALESCE(NEW.totals_snapshot_json, '') != COALESCE(OLD.totals_snapshot_json, '') OR
  COALESCE(NEW.conversion_snapshot_json, '') != COALESCE(OLD.conversion_snapshot_json, '') OR
  COALESCE(NEW.finalized_at, '') != COALESCE(OLD.finalized_at, '') OR
  COALESCE(NEW.locked_at, '') != COALESCE(OLD.locked_at, '')
)
BEGIN
  SELECT RAISE(ABORT, 'finalized invoices are immutable');
END;

