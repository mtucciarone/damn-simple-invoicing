<script lang="ts">
  import { onMount } from 'svelte';

  import { getAppState, getAppSettings, updateAppSettings } from '$lib/api/tauri';
  import SectionCard from '$lib/components/SectionCard.svelte';
  import type { AppBootState } from '$lib/types/domain';

  let appState: AppBootState | null = null;
  let settings = {
    invoice_number_prefix: 'INV',
    invoice_sequence: '1',
    reporting_currency_label: 'CAD',
    theme: 'dark',
  };
  let loading = true;
  let saving = false;
  let error: string | null = null;
  let notice: string | null = null;

  async function refreshSettings() {
    loading = true;
    error = null;
    try {
      const [nextState, nextSettings] = await Promise.all([getAppState(), getAppSettings()]);
      appState = nextState;
      settings = {
        invoice_number_prefix: nextSettings.invoice_number_prefix ?? 'INV',
        invoice_sequence: nextSettings.invoice_sequence ?? '1',
        reporting_currency_label: nextSettings.reporting_currency_label
          ?? nextSettings.default_currency_label
          ?? 'CAD',
        theme: nextSettings.theme ?? 'dark',
      };
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  async function saveSettings() {
    saving = true;
    error = null;
    notice = null;
    try {
      const nextSettings = await updateAppSettings({
        invoice_number_prefix: settings.invoice_number_prefix.trim() || 'INV',
        invoice_sequence: settings.invoice_sequence.trim() || '1',
        reporting_currency_label: settings.reporting_currency_label.trim() || 'CAD',
        theme: settings.theme.trim() || 'dark',
      });
      settings = {
        invoice_number_prefix: nextSettings.invoice_number_prefix ?? 'INV',
        invoice_sequence: nextSettings.invoice_sequence ?? '1',
        reporting_currency_label: nextSettings.reporting_currency_label
          ?? nextSettings.default_currency_label
          ?? 'CAD',
        theme: nextSettings.theme ?? 'dark',
      };
      notice = 'Settings saved locally.';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  onMount(async () => {
    await refreshSettings();
  });
</script>

<div class="space-y-6">
  <SectionCard
    title="Settings"
    eyebrow="App defaults"
    description="Persist local defaults such as invoice numbering, your tax/reporting currency label, and the saved theme preference."
  >
    <div class="grid gap-6 xl:grid-cols-[1fr_0.85fr]">
      <form class="space-y-4" on:submit|preventDefault={saveSettings}>
        <div class="grid gap-4 md:grid-cols-2">
          <label class="space-y-2">
            <span class="label">Invoice prefix</span>
            <input class="input-base" bind:value={settings.invoice_number_prefix} placeholder="INV" />
          </label>
          <label class="space-y-2">
            <span class="label">Invoice sequence</span>
            <input class="input-base" bind:value={settings.invoice_sequence} inputmode="numeric" />
          </label>
          <label class="space-y-2">
            <span class="label">Reporting currency label</span>
            <input class="input-base" bind:value={settings.reporting_currency_label} placeholder="CAD, USD, EUR, USDC, Credits" />
            <p class="text-xs text-slate-500">Any arbitrary label is valid. This is the currency used for reporting totals and payment conversion snapshots.</p>
          </label>
          <label class="space-y-2">
            <span class="label">Theme</span>
            <select class="input-base" bind:value={settings.theme}>
              <option value="dark">Dark</option>
              <option value="light">Light</option>
              <option value="system">System</option>
            </select>
          </label>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <button class="button-primary" disabled={saving} type="submit">Save settings</button>
          <p class="text-xs text-slate-500">
            The backend stores these values locally in SQLite under `app_settings`. The reporting currency drives tax totals and payment conversion snapshots.
          </p>
        </div>
      </form>

      <div class="space-y-4">
        <div class="panel-soft p-4">
          <p class="label">Local store</p>
          {#if loading}
            <p class="mt-3 text-sm text-slate-400">Loading settings from the local database...</p>
          {:else if appState}
            <div class="mt-3 space-y-2 text-sm text-slate-300">
              <p><span class="text-slate-500">Database:</span> <span class="break-all">{appState.databasePath}</span></p>
              <p><span class="text-slate-500">Active business:</span> {appState.activeBusiness?.businessName ?? 'None'}</p>
              <p><span class="text-slate-500">Reporting currency:</span> {appState.reportingCurrencyLabel}</p>
            </div>
          {/if}
        </div>

        <div class="panel-soft p-4 text-sm text-slate-400">
          Theme is persisted here so the app can evolve into a true preferences-driven desktop shell without introducing
          any cloud-backed profile state.
        </div>
      </div>
    </div>
  </SectionCard>

  {#if notice}
    <div class="rounded-2xl border border-emerald-400/20 bg-emerald-400/10 px-4 py-3 text-sm text-emerald-100">
      {notice}
    </div>
  {/if}
  {#if error}
    <div class="rounded-2xl border border-rose-400/20 bg-rose-400/10 px-4 py-3 text-sm text-rose-100">
      {error}
    </div>
  {/if}
</div>
