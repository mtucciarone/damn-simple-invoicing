INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES
  ('invoice_number_prefix', 'INV', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  ('invoice_sequence', '1', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  ('money_format', 'us', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  ('theme', 'dark', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  ('default_currency_label', '', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
