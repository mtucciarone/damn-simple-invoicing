use std::fs;
use std::path::Path;

use crate::error::AppResult;
use crate::models::{InvoiceDetail, InvoiceLineItem, Payment};

use super::money_display;

const PAGE_WIDTH: f32 = 595.28;
const PAGE_HEIGHT: f32 = 841.89;
const LEFT_MARGIN: f32 = 36.0;
const RIGHT_MARGIN: f32 = 36.0;
const TOP_MARGIN: f32 = 40.0;
const BOTTOM_MARGIN: f32 = 44.0;
const COLOR_HEADER_BG: (f32, f32, f32) = (0.07, 0.10, 0.17);
const COLOR_ACCENT: (f32, f32, f32) = (0.27, 0.55, 0.95);
const COLOR_PANEL: (f32, f32, f32) = (0.96, 0.97, 0.99);
const COLOR_PANEL_ALT: (f32, f32, f32) = (0.98, 0.99, 1.0);
const COLOR_BORDER: (f32, f32, f32) = (0.83, 0.87, 0.92);
const COLOR_TEXT: (f32, f32, f32) = (0.12, 0.15, 0.20);
const COLOR_MUTED: (f32, f32, f32) = (0.46, 0.51, 0.62);
const COLOR_SOFT: (f32, f32, f32) = (0.94, 0.96, 0.99);

pub fn write_invoice_pdf_document(detail: &InvoiceDetail, pdf_path: &Path) -> AppResult<()> {
    let mut document = PdfDocument::new(&format!("Invoice {}", detail.invoice.invoice_number));
    document.write_invoice(detail);
    fs::write(pdf_path, document.finish())?;
    Ok(())
}

struct PdfDocument {
    title: String,
    pages: Vec<String>,
    current: String,
    current_y: f32,
}

impl PdfDocument {
    fn new(title: &str) -> Self {
        let mut document = Self {
            title: title.to_string(),
            pages: Vec::new(),
            current: String::new(),
            current_y: PAGE_HEIGHT - TOP_MARGIN,
        };
        document.start_page();
        document
    }

    fn start_page(&mut self) {
        if !self.current.is_empty() {
            self.pages.push(std::mem::take(&mut self.current));
        }

        self.current_y = PAGE_HEIGHT - TOP_MARGIN;
        self.write_header();
    }

    fn write_header(&mut self) {
        let title = self.title.clone();
        let banner_height = 54.0;
        let banner_bottom = self.current_y - banner_height + 8.0;
        self.rect_fill_color(
            LEFT_MARGIN,
            banner_bottom,
            PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN,
            banner_height,
            COLOR_HEADER_BG,
        );
        self.rect_fill_color(LEFT_MARGIN, banner_bottom, 5.0, banner_height, COLOR_ACCENT);
        self.text_color(
            LEFT_MARGIN + 16.0,
            self.current_y - 17.0,
            18.0,
            "F2",
            &title,
            (1.0, 1.0, 1.0),
        );
        self.text_color(
            LEFT_MARGIN + 16.0,
            self.current_y - 33.0,
            9.0,
            "F1",
            "Generated locally from the offline invoice record.",
            (0.88, 0.91, 0.96),
        );
        self.current_y = banner_bottom - 16.0;
    }

    fn finish(mut self) -> Vec<u8> {
        if !self.current.is_empty() {
            self.pages.push(self.current);
        }
        build_pdf_bytes(&self.pages)
    }

    fn ensure_space(&mut self, required: f32) {
        if self.current_y - required < BOTTOM_MARGIN {
            self.start_page();
        }
    }

    fn text_color(&mut self, x: f32, y: f32, size: f32, font: &str, text: &str, color: (f32, f32, f32)) {
        self.current.push_str(&format!(
            "{:.3} {:.3} {:.3} rg\nBT /{font} {size:.2} Tf 1 0 0 1 {x:.2} {y:.2} Tm ({}) Tj ET\n",
            color.0,
            color.1,
            color.2,
            escape_pdf_text(text),
        ));
    }

    #[allow(dead_code)]
    fn text(&mut self, x: f32, y: f32, size: f32, font: &str, text: &str) {
        self.text_color(x, y, size, font, text, COLOR_TEXT);
    }

    fn line_color(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: (f32, f32, f32)) {
        self.current
            .push_str(&format!("{:.3} {:.3} {:.3} RG\n{x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S\n", color.0, color.1, color.2));
    }

    #[allow(dead_code)]
    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.line_color(x1, y1, x2, y2, COLOR_BORDER);
    }

    fn rect_fill_color(&mut self, x: f32, y: f32, width: f32, height: f32, color: (f32, f32, f32)) {
        self.current.push_str(&format!(
            "{:.3} {:.3} {:.3} rg\n{x:.2} {y:.2} {width:.2} {height:.2} re f\n",
            color.0, color.1, color.2
        ));
    }

    fn rect_fill_stroke_color(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill_color: (f32, f32, f32),
        stroke_color: (f32, f32, f32),
    ) {
        self.current.push_str(&format!(
            "{:.3} {:.3} {:.3} rg {:.3} {:.3} {:.3} RG\n{x:.2} {y:.2} {width:.2} {height:.2} re B\n",
            fill_color.0,
            fill_color.1,
            fill_color.2,
            stroke_color.0,
            stroke_color.1,
            stroke_color.2,
        ));
    }

    fn section_title(&mut self, title: &str) {
        self.ensure_space(24.0);
        let band_height = 18.0;
        let band_bottom = self.current_y - band_height + 4.0;
        self.rect_fill_color(
            LEFT_MARGIN,
            band_bottom,
            PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN,
            band_height,
            COLOR_SOFT,
        );
        self.rect_fill_color(LEFT_MARGIN, band_bottom, 4.0, band_height, COLOR_ACCENT);
        self.text_color(LEFT_MARGIN + 12.0, self.current_y, 11.0, "F2", title, COLOR_TEXT);
        self.current_y -= 20.0;
    }

    fn field(&mut self, label: &str, value: &str, width_chars: usize) {
        let lines = wrap_text(value, width_chars);
        self.ensure_space((lines.len() as f32 * 11.0) + 4.0);
        self.text_color(LEFT_MARGIN, self.current_y, 8.5, "F2", label, COLOR_MUTED);
        for line in lines {
            self.text_color(150.0, self.current_y, 8.5, "F1", &line, COLOR_TEXT);
            self.current_y -= 11.0;
        }
        self.current_y -= 2.0;
    }

    fn paragraph(&mut self, text: &str, width_chars: usize) {
        let lines = wrap_text(text, width_chars);
        self.ensure_space((lines.len() as f32 * 11.0) + 2.0);
        for line in lines {
            self.text_color(LEFT_MARGIN, self.current_y, 8.5, "F1", &line, COLOR_TEXT);
            self.current_y -= 11.0;
        }
        self.current_y -= 2.0;
    }

    #[allow(dead_code)]
    fn summary_value(&mut self, label: &str, value: &str) {
        self.field(label, value, 62);
    }

    fn metric_card(&mut self, x: f32, top_y: f32, width: f32, height: f32, label: &str, value: &str) {
        let bottom = top_y - height;
        self.rect_fill_stroke_color(x, bottom, width, height, COLOR_PANEL, COLOR_BORDER);
        self.text_color(x + 12.0, top_y - 12.0, 8.0, "F2", label, COLOR_MUTED);
        self.text_color(x + 12.0, top_y - 29.0, 13.0, "F2", value, COLOR_TEXT);
    }

    fn write_totals_cards(&mut self, detail: &InvoiceDetail) {
        self.ensure_space(102.0);
        let available_width = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN;
        let gap = 10.0;
        let card_width = (available_width - gap) / 2.0;
        let card_height = 40.0;
        let top_row = self.current_y;

        self.metric_card(
            LEFT_MARGIN,
            top_row,
            card_width,
            card_height,
            "Subtotal",
            &format!(
                "{} {}",
                money_display(detail.invoice.subtotal_minor),
                detail.invoice.currency_label
            ),
        );
        self.metric_card(
            LEFT_MARGIN + card_width + gap,
            top_row,
            card_width,
            card_height,
            "Paid",
            &format!(
                "{} {}",
                money_display(detail.invoice.paid_minor),
                detail.invoice.currency_label
            ),
        );

        self.current_y -= card_height + 10.0;
        let bottom_row = self.current_y;
        self.metric_card(
            LEFT_MARGIN,
            bottom_row,
            card_width,
            card_height,
            "Outstanding",
            &format!(
                "{} {}",
                money_display(detail.invoice.outstanding_minor),
                detail.invoice.currency_label
            ),
        );
        self.metric_card(
            LEFT_MARGIN + card_width + gap,
            bottom_row,
            card_width,
            card_height,
            "Total",
            &format!(
                "{} {}",
                money_display(detail.invoice.total_minor),
                detail.invoice.currency_label
            ),
        );
        self.current_y -= card_height + 8.0;
    }

    fn snapshot_card_height(&self, rows: &[(String, String)], value_width_chars: usize) -> f32 {
        let mut height = 18.0 + 26.0 + 8.0;
        for (label, value) in rows {
            let label_lines = wrap_text(label, 18);
            let value_lines = wrap_text(value, value_width_chars);
            let row_lines = label_lines.len().max(value_lines.len()).max(1) as f32;
            height += (row_lines * 11.0) + 6.0;
        }
        height + 6.0
    }

    fn draw_snapshot_card(
        &mut self,
        x: f32,
        top_y: f32,
        width: f32,
        title: &str,
        rows: &[(String, String)],
        value_width_chars: usize,
    ) -> f32 {
        let height = self.snapshot_card_height(rows, value_width_chars);
        let bottom = top_y - height;
        self.rect_fill_stroke_color(x, bottom, width, height, COLOR_PANEL, COLOR_BORDER);
        self.rect_fill_color(x, top_y - 14.0, width, 18.0, COLOR_SOFT);
        self.rect_fill_color(x, top_y - 14.0, 4.0, 18.0, COLOR_ACCENT);
        self.text_color(x + 12.0, top_y, 11.0, "F2", title, COLOR_TEXT);

        let mut cursor_y = top_y - 26.0;
        for (label, value) in rows {
            let label_lines = wrap_text(label, 18);
            let value_lines = wrap_text(value, value_width_chars);
            let row_lines = label_lines.len().max(value_lines.len()).max(1) as f32;
            let row_height = (row_lines * 11.0) + 5.0;
            self.text_color(x + 12.0, cursor_y, 8.0, "F2", &label_lines[0], COLOR_MUTED);
            for (index, line) in value_lines.iter().enumerate() {
                self.text_color(
                    x + 118.0,
                    cursor_y - (index as f32 * 11.0),
                    8.5,
                    "F1",
                    line,
                    COLOR_TEXT,
                );
            }
            cursor_y -= row_height;
        }

        height
    }

    fn write_party_snapshots(&mut self, detail: &InvoiceDetail) {
        self.ensure_space(160.0);
        let available_width = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN;
        let gap = 10.0;
        let card_width = (available_width - gap) / 2.0;
        let top_y = self.current_y;
        let business_rows = vec![
            ("Business name".to_string(), detail.invoice.business_snapshot.business_name.clone()),
            (
                "Legal name".to_string(),
                detail
                    .invoice
                    .business_snapshot
                    .legal_name
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Address".to_string(),
                detail
                    .invoice
                    .business_snapshot
                    .address
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Country".to_string(),
                detail
                    .invoice
                    .business_snapshot
                    .country
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Email".to_string(),
                detail
                    .invoice
                    .business_snapshot
                    .email
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Phone".to_string(),
                detail
                    .invoice
                    .business_snapshot
                    .phone
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Registration".to_string(),
                detail
                    .invoice
                    .business_snapshot
                    .registration_number
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Tax / VAT".to_string(),
                detail
                    .invoice
                    .business_snapshot
                    .tax_vat_number
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
        ];
        let client_rows = vec![
            ("Company name".to_string(), detail.invoice.client_snapshot.company_name.clone()),
            (
                "Contact person".to_string(),
                detail
                    .invoice
                    .client_snapshot
                    .contact_person
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Email".to_string(),
                detail
                    .invoice
                    .client_snapshot
                    .email
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Address".to_string(),
                detail
                    .invoice
                    .client_snapshot
                    .address
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Country".to_string(),
                detail
                    .invoice
                    .client_snapshot
                    .country
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
            (
                "Notes".to_string(),
                detail
                    .invoice
                    .client_snapshot
                    .notes
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
        ];

        let business_height = self.draw_snapshot_card(
            LEFT_MARGIN,
            top_y,
            card_width,
            "Business snapshot",
            &business_rows,
            22,
        );
        let client_height = self.draw_snapshot_card(
            LEFT_MARGIN + card_width + gap,
            top_y,
            card_width,
            "Client snapshot",
            &client_rows,
            22,
        );
        self.current_y -= business_height.max(client_height) + 12.0;
    }

    fn payment_card_height(&self, payment: &Payment) -> f32 {
        let mut height = 86.0;
        let reference_lines = wrap_text(payment.transaction_reference_id.as_deref().unwrap_or("-"), 34);
        let note_lines = payment
            .notes
            .as_deref()
            .map(|notes| wrap_text(notes, 42).len())
            .unwrap_or(0);
        height += ((reference_lines.len() + note_lines).max(1) as f32) * 11.0;
        height
    }

    fn draw_payment_card(&mut self, payment: &Payment, _invoice_currency: &str) {
        let card_width = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN;
        let card_height = self.payment_card_height(payment);
        self.ensure_space(card_height + 10.0);

        let top_y = self.current_y;
        let bottom = top_y - card_height;
        self.rect_fill_stroke_color(LEFT_MARGIN, bottom, card_width, card_height, COLOR_PANEL, COLOR_BORDER);
        self.rect_fill_color(LEFT_MARGIN, top_y - 14.0, card_width, 18.0, COLOR_SOFT);
        self.rect_fill_color(LEFT_MARGIN, top_y - 14.0, 4.0, 18.0, COLOR_ACCENT);
        self.text_color(
            LEFT_MARGIN + 12.0,
            top_y,
            11.0,
            "F2",
            &format!("{} · {}", payment.payment_date, payment.payment_source.as_str()),
            COLOR_TEXT,
        );

        let left_x = LEFT_MARGIN + 12.0;
        let right_x = LEFT_MARGIN + card_width - 154.0;
        let mut cursor_y = top_y - 26.0;
        let reference = payment.transaction_reference_id.as_deref().unwrap_or("-");
        let reference_lines = wrap_text(reference, 40);

        self.text_color(left_x, cursor_y, 8.0, "F2", "Invoice", COLOR_MUTED);
        self.text_color(
            left_x + 84.0,
            cursor_y,
            8.5,
            "F1",
            payment.invoice_number.as_deref().unwrap_or(&format!("#{}", payment.invoice_id)),
            COLOR_TEXT,
        );
        cursor_y -= 12.0;
        self.text_color(left_x, cursor_y, 8.0, "F2", "Client", COLOR_MUTED);
        self.text_color(
            left_x + 84.0,
            cursor_y,
            8.5,
            "F1",
            payment.client_company_name.as_deref().unwrap_or("-"),
            COLOR_TEXT,
        );
        cursor_y -= 12.0;
        self.text_color(left_x, cursor_y, 8.0, "F2", "Reference", COLOR_MUTED);
        for (index, line) in reference_lines.iter().enumerate() {
            self.text_color(left_x + 84.0, cursor_y - (index as f32 * 11.0), 8.5, "F1", line, COLOR_TEXT);
        }
        cursor_y -= ((reference_lines.len().max(1) as f32) * 11.0) + 4.0;
        self.text_color(left_x, cursor_y, 8.0, "F2", "Rate", COLOR_MUTED);
        self.text_color(
            left_x + 84.0,
            cursor_y,
            8.5,
            "F1",
            payment.conversion_rate.as_deref().unwrap_or("-"),
            COLOR_TEXT,
        );
        cursor_y -= 12.0;
        if let Some(notes) = payment.notes.as_deref() {
            let note_lines = wrap_text(notes, 40);
            self.text_color(left_x, cursor_y, 8.0, "F2", "Notes", COLOR_MUTED);
            for (index, line) in note_lines.iter().enumerate() {
                self.text_color(left_x + 84.0, cursor_y - (index as f32 * 11.0), 8.5, "F1", line, COLOR_TEXT);
            }
        }

        self.metric_card(
            right_x,
            top_y - 18.0,
            142.0,
            34.0,
            "Source amount",
            &format!(
                "{} {}",
                money_display(payment.amount_minor),
                payment.currency_label
            ),
        );
        self.metric_card(
            right_x,
            top_y - 60.0,
            142.0,
            34.0,
            "Reporting amount",
            &format!(
                "{} {}",
                money_display(payment.converted_amount_minor.unwrap_or(payment.amount_minor)),
                payment.reporting_currency_label
            ),
        );

        self.current_y -= card_height + 8.0;
    }

    fn write_payment_cards(&mut self, payments: &[Payment], invoice_currency: &str) {
        if payments.is_empty() {
            self.paragraph("No payments recorded for this invoice yet.", 78);
            return;
        }

        for payment in payments {
            self.draw_payment_card(payment, invoice_currency);
        }
        self.current_y -= 6.0;
    }

    fn conversion_card_height(&self, conversion: &crate::models::CurrencyConversion) -> f32 {
        let rate_lines = wrap_text(&conversion.conversion_rate, 26).len();
        80.0 + (rate_lines as f32 * 11.0)
    }

    fn draw_conversion_card(&mut self, conversion: &crate::models::CurrencyConversion) {
        let card_width = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN;
        let card_height = self.conversion_card_height(conversion);
        self.ensure_space(card_height + 10.0);

        let top_y = self.current_y;
        let bottom = top_y - card_height;
        self.rect_fill_stroke_color(LEFT_MARGIN, bottom, card_width, card_height, COLOR_PANEL, COLOR_BORDER);
        self.rect_fill_color(LEFT_MARGIN, top_y - 14.0, card_width, 18.0, COLOR_SOFT);
        self.rect_fill_color(LEFT_MARGIN, top_y - 14.0, 4.0, 18.0, COLOR_ACCENT);
        self.text_color(
            LEFT_MARGIN + 12.0,
            top_y,
            11.0,
            "F2",
            &format!(
                "{} → {}",
                conversion.source_currency_label, conversion.target_currency_label
            ),
            COLOR_TEXT,
        );

        let left_x = LEFT_MARGIN + 12.0;
        let right_x = LEFT_MARGIN + card_width - 154.0;
        let mut cursor_y = top_y - 26.0;
        self.text_color(left_x, cursor_y, 8.0, "F2", "Rate", COLOR_MUTED);
        self.text_color(
            left_x + 84.0,
            cursor_y,
            8.5,
            "F1",
            &conversion.conversion_rate,
            COLOR_TEXT,
        );
        cursor_y -= 12.0;
        self.text_color(left_x, cursor_y, 8.0, "F2", "Source amount", COLOR_MUTED);
        self.text_color(
            left_x + 84.0,
            cursor_y,
            8.5,
            "F1",
            &format!(
                "{} {}",
                money_display(conversion.source_amount_minor),
                conversion.source_currency_label
            ),
            COLOR_TEXT,
        );
        cursor_y -= 12.0;
        self.text_color(left_x, cursor_y, 8.0, "F2", "Captured", COLOR_MUTED);
        self.text_color(
            left_x + 84.0,
            cursor_y,
            8.5,
            "F1",
            &conversion.captured_at,
            COLOR_TEXT,
        );

        self.metric_card(
            right_x,
            top_y - 18.0,
            142.0,
            34.0,
            "Converted amount",
            &format!(
                "{} {}",
                money_display(conversion.converted_amount_minor),
                conversion.target_currency_label
            ),
        );
        self.metric_card(
            right_x,
            top_y - 60.0,
            142.0,
            34.0,
            "From / To",
            &format!(
                "{} → {}",
                conversion.source_currency_label, conversion.target_currency_label
            ),
        );

        self.current_y -= card_height + 8.0;
    }

    fn write_conversion_cards(&mut self, conversions: &[crate::models::CurrencyConversion]) {
        if conversions.is_empty() {
            self.paragraph("No conversion snapshots stored for this invoice yet.", 78);
            return;
        }

        for conversion in conversions {
            self.draw_conversion_card(conversion);
        }
        self.current_y -= 6.0;
    }

    fn write_invoice(&mut self, detail: &InvoiceDetail) {
        self.section_title("Invoice summary");
        self.field("Invoice number", &detail.invoice.invoice_number, 68);
        self.field("Status", detail.invoice.status.as_str(), 20);
        self.field("Issue date", &detail.invoice.issue_date, 20);
        self.field("Due date", &detail.invoice.due_date, 20);
        self.field(
            "Currency",
            &format!("{} (label only)", detail.invoice.currency_label),
            30,
        );

        self.section_title("Business and client snapshots");
        self.write_party_snapshots(detail);

        self.section_title("Totals");
        self.write_totals_cards(detail);

        self.section_title("Line items");
        self.write_line_items(&detail.line_items, &detail.invoice.currency_label);

        if !detail.payments.is_empty() {
            self.section_title("Payments");
            self.write_payment_cards(&detail.payments, &detail.invoice.currency_label);
        }

        if !detail.conversions.is_empty() {
            self.current_y -= 8.0;
            self.section_title("Currency conversions");
            self.write_conversion_cards(&detail.conversions);
        }

        if let Some(notes) = detail.invoice.notes.as_deref() {
            self.section_title("Notes");
            self.paragraph(notes, 78);
        }

        if let Some(payment_terms) = detail.invoice.payment_terms.as_deref() {
            self.section_title("Payment terms");
            self.paragraph(payment_terms, 78);
        }
    }

    fn write_line_items(&mut self, items: &[InvoiceLineItem], currency: &str) {
        if items.is_empty() {
            self.paragraph("No line items were stored on this invoice.", 78);
            return;
        }

        self.ensure_space(22.0);
        let table_width = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN;
        self.rect_fill_color(LEFT_MARGIN, self.current_y - 12.0, table_width, 16.0, COLOR_PANEL);
        self.text_color(LEFT_MARGIN, self.current_y, 8.0, "F2", "Description", COLOR_MUTED);
        self.text_color(320.0, self.current_y, 8.0, "F2", "Qty", COLOR_MUTED);
        self.text_color(390.0, self.current_y, 8.0, "F2", "Rate", COLOR_MUTED);
        self.text_color(495.0, self.current_y, 8.0, "F2", "Total", COLOR_MUTED);
        self.current_y -= 9.0;
        self.line_color(LEFT_MARGIN, self.current_y, PAGE_WIDTH - RIGHT_MARGIN, self.current_y, COLOR_BORDER);
        self.current_y -= 10.0;

        for (index, item) in items.iter().enumerate() {
            let description_lines = wrap_text(&item.description, 42);
            let row_height = (description_lines.len().max(1) as f32 * 11.0) + 4.0;
            self.ensure_space(row_height + 8.0);
            let row_top = self.current_y;
            let row_bottom = row_top - row_height + 3.0;
            let row_fill = if index % 2 == 0 { COLOR_PANEL_ALT } else { (1.0, 1.0, 1.0) };
            self.rect_fill_color(LEFT_MARGIN, row_bottom, table_width, row_height - 2.0, row_fill);
            for (index, line) in description_lines.iter().enumerate() {
                self.text_color(LEFT_MARGIN, row_top - (index as f32 * 11.0), 8.5, "F1", line, COLOR_TEXT);
            }
            self.text_color(320.0, row_top, 8.5, "F1", &item.quantity, COLOR_TEXT);
            self.text_color(
                390.0,
                row_top,
                8.5,
                "F1",
                &format!("{} {}", money_display(item.rate_minor), currency),
                COLOR_TEXT,
            );
            self.text_color(
                495.0,
                row_top,
                8.5,
                "F1",
                &format!("{} {}", money_display(item.line_total_minor), currency),
                COLOR_TEXT,
            );
            self.current_y -= row_height;
            self.line_color(LEFT_MARGIN, self.current_y, PAGE_WIDTH - RIGHT_MARGIN, self.current_y, COLOR_BORDER);
            self.current_y -= 6.0;
        }
    }

    #[allow(dead_code)]
    fn write_payments(&mut self, payments: &[Payment], _invoice_currency: &str) {
        self.ensure_space(22.0);
        let table_width = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN;
        self.rect_fill_color(LEFT_MARGIN, self.current_y - 12.0, table_width, 16.0, COLOR_PANEL);
        self.text_color(LEFT_MARGIN, self.current_y, 8.0, "F2", "Date", COLOR_MUTED);
        self.text_color(140.0, self.current_y, 8.0, "F2", "Source", COLOR_MUTED);
        self.text_color(300.0, self.current_y, 8.0, "F2", "Paid", COLOR_MUTED);
        self.text_color(390.0, self.current_y, 8.0, "F2", "Reporting", COLOR_MUTED);
        self.text_color(500.0, self.current_y, 8.0, "F2", "Reference", COLOR_MUTED);
        self.current_y -= 9.0;
        self.line_color(LEFT_MARGIN, self.current_y, PAGE_WIDTH - RIGHT_MARGIN, self.current_y, COLOR_BORDER);
        self.current_y -= 10.0;

        for (index, payment) in payments.iter().enumerate() {
            let reference = payment.transaction_reference_id.as_deref().unwrap_or("-");
            let reference_lines = wrap_text(reference, 24);
            let row_height = (reference_lines.len().max(1) as f32 * 11.0) + 4.0;
            self.ensure_space(row_height + 8.0);
            let row_top = self.current_y;
            let row_bottom = row_top - row_height + 3.0;
            let row_fill = if index % 2 == 0 { COLOR_PANEL_ALT } else { (1.0, 1.0, 1.0) };
            self.rect_fill_color(LEFT_MARGIN, row_bottom, table_width, row_height - 2.0, row_fill);
            self.text_color(LEFT_MARGIN, row_top, 8.5, "F1", &payment.payment_date, COLOR_TEXT);
            self.text_color(
                140.0,
                row_top,
                8.5,
                "F1",
                payment.payment_source.as_str(),
                COLOR_TEXT,
            );
            self.text_color(
                300.0,
                row_top,
                8.5,
                "F1",
                &format!(
                    "{} {}",
                    money_display(payment.amount_minor),
                    payment.currency_label
                ),
                COLOR_TEXT,
            );
            self.text_color(
                390.0,
                row_top,
                8.5,
                "F1",
                &format!(
                    "{} {}",
                    money_display(payment.converted_amount_minor.unwrap_or(payment.amount_minor)),
                    payment.reporting_currency_label
                ),
                COLOR_TEXT,
            );
            for (index, line) in reference_lines.iter().enumerate() {
                self.text_color(500.0, row_top - (index as f32 * 11.0), 8.5, "F1", line, COLOR_TEXT);
            }
            self.current_y -= row_height;
            self.line_color(LEFT_MARGIN, self.current_y, PAGE_WIDTH - RIGHT_MARGIN, self.current_y, COLOR_BORDER);
            self.current_y -= 6.0;
        }
    }

    #[allow(dead_code)]
    fn write_conversions(&mut self, conversions: &[crate::models::CurrencyConversion]) {
        self.ensure_space(22.0);
        let table_width = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN;
        self.rect_fill_color(LEFT_MARGIN, self.current_y - 12.0, table_width, 16.0, COLOR_PANEL);
        self.text_color(LEFT_MARGIN, self.current_y, 8.0, "F2", "Source", COLOR_MUTED);
        self.text_color(190.0, self.current_y, 8.0, "F2", "Target", COLOR_MUTED);
        self.text_color(310.0, self.current_y, 8.0, "F2", "Rate", COLOR_MUTED);
        self.text_color(440.0, self.current_y, 8.0, "F2", "Converted", COLOR_MUTED);
        self.current_y -= 9.0;
        self.line_color(LEFT_MARGIN, self.current_y, PAGE_WIDTH - RIGHT_MARGIN, self.current_y, COLOR_BORDER);
        self.current_y -= 10.0;

        for (index, conversion) in conversions.iter().enumerate() {
            self.ensure_space(20.0);
            let row_top = self.current_y;
            let row_bottom = row_top - 16.0 + 3.0;
            let row_fill = if index % 2 == 0 { COLOR_PANEL_ALT } else { (1.0, 1.0, 1.0) };
            self.rect_fill_color(LEFT_MARGIN, row_bottom, table_width, 14.0, row_fill);
            self.text_color(
                LEFT_MARGIN,
                row_top,
                8.5,
                "F1",
                &conversion.source_currency_label,
                COLOR_TEXT,
            );
            self.text_color(
                190.0,
                row_top,
                8.5,
                "F1",
                &conversion.target_currency_label,
                COLOR_TEXT,
            );
            self.text_color(310.0, row_top, 8.5, "F1", &conversion.conversion_rate, COLOR_TEXT);
            self.text_color(
                440.0,
                row_top,
                8.5,
                "F1",
                &format!(
                    "{} {}",
                    money_display(conversion.converted_amount_minor),
                    conversion.target_currency_label
                ),
                COLOR_TEXT,
            );
            self.line_color(LEFT_MARGIN, row_top - 14.0, PAGE_WIDTH - RIGHT_MARGIN, row_top - 14.0, COLOR_BORDER);
            self.current_y -= 14.0;
        }
    }
}

fn build_pdf_bytes(pages: &[String]) -> Vec<u8> {
    let page_count = pages.len();
    let object_count = 4 + page_count * 2;
    let mut objects = Vec::with_capacity(object_count);

    objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());

    let kids = (0..page_count)
        .map(|index| format!("{} 0 R", 5 + index * 2))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!(
        "<< /Type /Pages /Kids [ {kids} ] /Count {page_count} >>"
    ));
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string());
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_string());

    for (index, content) in pages.iter().enumerate() {
        let page_object_id = 5 + index * 2;
        let content_object_id = page_object_id + 1;
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH:.2} {PAGE_HEIGHT:.2}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {content_object_id} 0 R >>"
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.as_bytes().len(),
            content
        ));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"%PDF-1.4\n%\xFF\xFF\xFF\xFF\n");

    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object_body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        bytes.extend_from_slice(object_body.as_bytes());
        bytes.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );

    bytes
}

fn sanitize_text(input: &str) -> String {
    input
        .chars()
        .map(|character| if character.is_ascii() { character } else { '?' })
        .collect()
}

fn escape_pdf_text(input: &str) -> String {
    sanitize_text(input)
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

fn wrap_text(input: &str, width_chars: usize) -> Vec<String> {
    let sanitized = sanitize_text(input);
    let trimmed = sanitized.trim();

    if trimmed.is_empty() {
        return vec!["-".to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for raw_word in trimmed.split_whitespace() {
        let mut word = raw_word.to_string();

        while word.len() > width_chars {
            let split = word[..width_chars].to_string();
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            lines.push(split);
            word = word[width_chars..].to_string();
        }

        if current.is_empty() {
            current = word;
        } else if current.len() + 1 + word.len() <= width_chars {
            current.push(' ');
            current.push_str(&word);
        } else {
            lines.push(std::mem::take(&mut current));
            current = word;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push("-".to_string());
    }

    lines
}
