import { derived, writable } from 'svelte/store';

import { moneyFormatFromSetting, type MoneyFormatStyle } from '$lib/utils/money';

export const appSettings = writable<Record<string, string>>({});

export const moneyFormatPreference = derived(appSettings, ($settings): MoneyFormatStyle =>
  moneyFormatFromSetting($settings.money_format),
);

export function setAppSettings(settings: Record<string, string>) {
  appSettings.set(settings);
}
