INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES
  ('reporting_currency_label', 'CAD', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
