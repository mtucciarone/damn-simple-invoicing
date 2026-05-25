# Architecture

## Product Shape

This is a local-first, single-user invoicing desktop app. Every source of truth lives in SQLite on the local machine. The frontend is a static SvelteKit app running inside a Tauri shell, and Rust owns persistence, calculations, backup, CSV export, and PDF generation.

## Constraints

- Windows, macOS, and Linux support
- No cloud sync
- No accounts
- No telemetry
- No online APIs
- No floating point math for money
- Finalized invoices are immutable
- Currency labels are arbitrary strings

## Recommended Folder Structure

```text
damn-simple-invoicing/
├─ README.md
├─ package.json
├─ svelte.config.js
├─ vite.config.ts
├─ tailwind.config.cjs
├─ postcss.config.cjs
├─ tsconfig.json
├─ src/
│  ├─ app.html
│  ├─ app.css
│  ├─ app.d.ts
│  ├─ lib/
│  │  ├─ api/
│  │  ├─ components/
│  │  ├─ types/
│  │  └─ utils/
│  └─ routes/
└─ src-tauri/
   ├─ Cargo.toml
   ├─ build.rs
   ├─ tauri.conf.json
   ├─ capabilities/
   ├─ migrations/
   └─ src/
      ├─ commands/
      ├─ db/
      ├─ models/
      ├─ services/
      ├─ error.rs
      ├─ state.rs
      └─ main.rs
```

## SQLite Schema

### `businesses`

Stores the business profile shown on invoices and snapshots.

- `id`
- `is_active`
- `business_name`
- `legal_name`
- `address`
- `country`
- `email`
- `phone`
- `registration_number`
- `tax_vat_number`
- `logo_path`
- `archived_at`
- `created_at`
- `updated_at`

Notes:

- Only one row should be active at a time.
- Invoice creation snapshots the currently selected business profile.

### `clients`

Stores client/customer records.

- `id`
- `company_name`
- `contact_person`
- `email`
- `address`
- `country`
- `notes`
- `archived_at`
- `created_at`
- `updated_at`

Notes:

- Soft archive is preferred over hard delete.
- Each invoice keeps a client snapshot so later edits do not rewrite history.

### `invoices`

Stores invoice headers and snapshot data.

- `id`
- `invoice_number`
- `business_id`
- `client_id`
- `status`
- `issue_date`
- `due_date`
- `currency_label`
- `notes`
- `payment_terms`
- `subtotal_minor`
- `total_minor`
- `paid_minor`
- `outstanding_minor`
- `finalized_at`
- `locked_at`
- `cancelled_at`
- `business_snapshot_json`
- `client_snapshot_json`
- `totals_snapshot_json`
- `conversion_snapshot_json`
- `created_at`
- `updated_at`

Notes:

- `status` is one of Draft, Sent, Partially Paid, Paid, Overdue, Cancelled.
- Finalized invoices are locked against edits to business data, client selection, dates, notes, terms, and line items.
- Status/payment summary fields can still be updated by backend logic.

### `invoice_line_items`

Stores invoice rows.

- `id`
- `invoice_id`
- `position`
- `description`
- `quantity`
- `rate_minor`
- `line_total_minor`
- `created_at`
- `updated_at`

Notes:

- `quantity` is stored as a decimal string so hours and partial units work without floats.
- `line_total_minor` is computed in Rust using `rust_decimal`.

### `payments`

Stores payments attached to invoices.

- `id`
- `invoice_id`
- `amount_minor`
- `currency_label`
- `converted_amount_minor`
- `payment_date`
- `payment_source`
- `transaction_reference_id`
- `notes`
- `created_at`
- `updated_at`

Notes:

- Partial payments are supported.
- Invoice payment status is recomputed after payment CRUD changes.

### `currency_conversions`

Stores conversion snapshots and permanent rate history.

- `id`
- `invoice_id`
- `payment_id`
- `source_currency_label`
- `target_currency_label`
- `conversion_rate`
- `source_amount_minor`
- `converted_amount_minor`
- `captured_at`

Notes:

- Conversion rates are manual only.
- Rates are stored as decimal strings, not floats.

### `app_settings`

Stores application-level key/value settings.

- `key`
- `value`
- `updated_at`

Suggested keys:

- `active_business_id`
- `invoice_number_prefix`
- `invoice_sequence`
- `default_currency_label`
- `theme`

## Rust Backend Architecture

### `db`

Responsibilities:

- Resolve the local SQLite file path inside the app data directory
- Open connections
- Run migrations
- Enable foreign keys
- Provide backup/restore helpers

### `models`

Responsibilities:

- Typed Rust domain objects shared with Tauri commands
- Money model with integer minor units
- Snapshot structs for invoice freezing
- Dashboard/report types

### `services`

Responsibilities:

- Invoice calculations
- Status recomputation
- Snapshot generation
- HTML invoice templating
- PDF rendering
- CSV export
- Backup and restore

### `commands`

Responsibilities:

- Expose a small, typed IPC surface to the frontend
- Validate inputs
- Delegate to repositories and services

## Tauri Command Surface

| Command | Responsibility |
| --- | --- |
| `get_app_state` | Load startup metadata for the frontend |
| `list_businesses` | Read all business profiles |
| `create_business` | Insert a business profile |
| `update_business` | Update a business profile |
| `archive_business` | Soft archive a business profile |
| `set_active_business` | Switch the active business profile |
| `get_active_business` | Return the current active business profile |
| `list_clients` | Search and filter clients |
| `get_client` | Read a single client with invoice history |
| `create_client` | Insert a client |
| `update_client` | Update a client |
| `archive_client` | Soft archive a client |
| `delete_client` | Hard delete a draft-safe client |
| `list_invoices` | Search and filter invoices |
| `get_invoice` | Read invoice header, line items, payments, conversions |
| `create_invoice` | Create a draft invoice with snapshots |
| `update_invoice` | Edit a draft invoice |
| `duplicate_invoice` | Clone an invoice as a new draft |
| `delete_draft_invoice` | Remove a draft invoice |
| `finalize_invoice` | Lock a draft invoice |
| `reorder_invoice_line_items` | Persist line item order |
| `record_payment` | Add a payment and refresh invoice status |
| `update_payment` | Edit a payment and refresh invoice status |
| `delete_payment` | Remove a payment and refresh invoice status |
| `list_payments` | Search and filter payments |
| `get_dashboard_summary` | Aggregate dashboard metrics |
| `export_invoice_pdf` | Render invoice HTML and export a PDF |
| `open_invoice_pdf` | Export and open the PDF file locally |
| `export_database_backup` | Copy the SQLite database to a backup path |
| `restore_database_backup` | Replace the current database from a backup |
| `export_invoices_csv` | Export invoice rows to CSV |
| `export_payments_csv` | Export payment rows to CSV |
| `get_app_settings` | Read application settings |
| `update_app_settings` | Update application settings |

## Frontend Architecture

### `routes`

The app uses route-based sections:

- Dashboard
- Business profile
- Clients
- Invoices
- Payments
- Reports
- Backups
- Settings

### `lib/components`

Reusable UI pieces:

- application shell
- metric cards
- section cards
- status badges
- money formatting
- empty states

### `lib/api`

Typed wrappers around `invoke()` calls so page code never hardcodes raw command names.

### `lib/types`

Shared frontend types that mirror the Rust IPC models.

## Initial Migrations

### `0001_init.sql`

Creates:

- `schema_migrations`
- `businesses`
- `clients`
- `invoices`
- `invoice_line_items`
- `payments`
- `currency_conversions`
- `app_settings`

Also creates:

- indexes for search and reporting
- immutable-invoice triggers
- line-item immutability triggers

### `0002_seed_settings.sql`

Seeds:

- invoice numbering defaults
- app theme default
- empty default currency label placeholder

## Core Money Model

```rust
pub struct Money {
    pub amount_minor: i64,
    pub currency: String,
}
```

Rules:

- no floats
- calculations use `rust_decimal`
- display uses fixed precision formatting in the UI

## MVP Implementation Plan

1. Business profile CRUD
2. Client CRUD with search and archive
3. Draft invoice creation with line items
4. Finalize invoice locking and snapshot capture
5. Payment CRUD and status recomputation
6. Dashboard metrics and recent activity
7. CSV export
8. Backup and restore
9. PDF export
10. Route-level polish and keyboard shortcuts

## Notes on PDF Export

The PDF pipeline should be:

1. Load invoice data from SQLite
2. Render a local HTML template
3. Print that HTML to a PDF file using a local renderer
4. Open the resulting file locally if requested

The renderer adapter should stay isolated so the app can switch implementations later without changing invoice templates.

