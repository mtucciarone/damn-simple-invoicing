<script lang="ts">
  import { onMount } from 'svelte';

  import {
    exportInvoicesCsv,
    exportPaymentsCsv,
    getDashboardSummary,
    listClients,
  } from '$lib/api/tauri';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import MetricCard from '$lib/components/MetricCard.svelte';
  import MoneyValue from '$lib/components/MoneyValue.svelte';
  import SectionCard from '$lib/components/SectionCard.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import type { Client, DashboardFilters, DashboardSummary } from '$lib/types/domain';
  import { blankToNull } from '$lib/utils/money';

  let dashboard: DashboardSummary | null = null;
  let clients: Client[] = [];
  let filters = {
    fromDate: '',
    toDate: '',
    clientId: '',
    currencyLabel: '',
  };
  let invoiceCsvPath = 'exports/invoices.csv';
  let paymentCsvPath = 'exports/payments.csv';
  let loading = true;
  let exporting = false;
  let error: string | null = null;
  let notice: string | null = null;

  function buildFilters(): DashboardFilters {
    return {
      fromDate: blankToNull(filters.fromDate),
      toDate: blankToNull(filters.toDate),
      clientId: filters.clientId ? Number.parseInt(filters.clientId, 10) : null,
      currencyLabel: blankToNull(filters.currencyLabel),
    };
  }

  async function refreshReport() {
    loading = true;
    error = null;
    try {
      const [nextDashboard, nextClients] = await Promise.all([
        getDashboardSummary(buildFilters()),
        listClients(null, true),
      ]);
      dashboard = nextDashboard;
      clients = nextClients;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  async function exportInvoices() {
    exporting = true;
    error = null;
    notice = null;
    try {
      const result = await exportInvoicesCsv(invoiceCsvPath, buildFilters());
      notice = `Invoices CSV written to ${result.path}`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      exporting = false;
    }
  }

  async function exportPayments() {
    exporting = true;
    error = null;
    notice = null;
    try {
      const result = await exportPaymentsCsv(paymentCsvPath, buildFilters());
      notice = `Payments CSV written to ${result.path}`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      exporting = false;
    }
  }

  onMount(async () => {
    await refreshReport();
  });
</script>

<div class="space-y-6">
  <SectionCard
    title="Reporting"
    eyebrow="Dashboard filters"
    description="Date, client, and currency filters are pushed into the Rust summary query so all aggregates stay local."
  >
    <svelte:fragment slot="actions">
      <button class="button-secondary" disabled={loading || exporting} on:click={refreshReport} type="button">Refresh</button>
    </svelte:fragment>

    <div class="space-y-4">
      <div class="panel-soft p-4">
        <div class="grid gap-4 lg:grid-cols-4">
          <label class="space-y-2">
            <span class="label">From date</span>
            <input class="input-base" bind:value={filters.fromDate} type="date" />
          </label>
          <label class="space-y-2">
            <span class="label">To date</span>
            <input class="input-base" bind:value={filters.toDate} type="date" />
          </label>
          <label class="space-y-2">
            <span class="label">Client</span>
            <select class="input-base" bind:value={filters.clientId}>
              <option value="">All clients</option>
              {#each clients as client}
                <option value={String(client.id)}>{client.companyName}</option>
              {/each}
            </select>
          </label>
          <label class="space-y-2">
            <span class="label">Currency</span>
            <input class="input-base" bind:value={filters.currencyLabel} placeholder="USD" />
          </label>
        </div>

        <div class="mt-4 flex flex-wrap items-center gap-3">
          <button class="button-primary" disabled={loading || exporting} on:click={refreshReport} type="button">Apply filters</button>
          <p class="text-xs text-slate-500">All results come from SQLite summary queries.</p>
        </div>
      </div>

      {#if loading}
        <SectionCard title="Dashboard" eyebrow="Loading" description="Reading local aggregates from SQLite...">
          <p class="text-sm text-slate-400">The Rust backend is building report metrics from the local database.</p>
        </SectionCard>
      {:else if error}
        <EmptyState
          title="Reporting unavailable"
          description={`The app could not build the local summary. ${error}`}
          actionLabel="Retry"
          href="/reports"
        />
      {:else if !dashboard}
        <EmptyState
          title="No reporting data"
          description="Create a business profile, add clients, issue invoices, and record payments to populate the dashboard."
          actionLabel="Go to invoices"
          href="/invoices"
        />
      {:else}
        <section class="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
          <MetricCard label="Total invoiced" value={(dashboard.totalInvoicedMinor / 100).toFixed(2)} detail="All non-draft invoices" tone="accent" />
          <MetricCard label="Source paid" value={(dashboard.totalPaidMinor / 100).toFixed(2)} detail="Raw payment amounts in their source currencies" />
          <MetricCard label={`Reporting total (${dashboard.reportingCurrencyLabel})`} value={`${(dashboard.reportedIncomeMinor / 100).toFixed(2)} ${dashboard.reportingCurrencyLabel}`} detail="All payments converted to the configured reporting currency" tone="warm" />
          <MetricCard label="Outstanding" value={(dashboard.outstandingBalanceMinor / 100).toFixed(2)} detail="Open invoice balance" tone="warm" />
          <MetricCard label="Invoice count" value={String(dashboard.invoiceCount)} detail={`${dashboard.overdueInvoiceCount} overdue`} />
        </section>

        <div class="grid gap-6 xl:grid-cols-[1.2fr_0.9fr]">
          <SectionCard title="Recent invoices" eyebrow="Activity" description="Newest invoices from the local ledger.">
            {#if dashboard.recentInvoices.length === 0}
              <p class="text-sm text-slate-400">No invoices yet.</p>
            {:else}
              <div class="overflow-hidden rounded-2xl border border-white/10">
                <table class="min-w-full divide-y divide-white/10 text-sm">
                  <thead class="bg-white/[0.03] text-left text-xs uppercase tracking-[0.16em] text-slate-400">
                    <tr>
                      <th class="px-4 py-3">Invoice</th>
                      <th class="px-4 py-3">Client</th>
                      <th class="px-4 py-3">Status</th>
                      <th class="px-4 py-3">Due</th>
                      <th class="px-4 py-3 text-right">Total</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-white/10">
                    {#each dashboard.recentInvoices as invoice}
                      <tr class="bg-white/[0.02]">
                        <td class="px-4 py-3 font-medium text-white">{invoice.invoiceNumber}</td>
                        <td class="px-4 py-3 text-slate-300">{invoice.clientCompanyName}</td>
                        <td class="px-4 py-3"><StatusBadge status={invoice.status} /></td>
                        <td class="px-4 py-3 text-slate-300">{invoice.dueDate}</td>
                        <td class="px-4 py-3 text-right"><MoneyValue amountMinor={invoice.totalMinor} currency={invoice.currencyLabel} /></td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}
          </SectionCard>

          <div class="space-y-6">
            <SectionCard title="Overdue invoices" eyebrow="Risk" description="Invoices that need immediate attention.">
              {#if dashboard.overdueInvoices.length === 0}
                <p class="text-sm text-slate-400">No overdue invoices.</p>
              {:else}
                <div class="space-y-3">
                  {#each dashboard.overdueInvoices as invoice}
                    <div class="flex items-center justify-between gap-4 rounded-2xl border border-rose-400/20 bg-rose-400/10 px-4 py-3">
                      <div>
                        <p class="font-medium text-white">{invoice.invoiceNumber}</p>
                        <p class="text-xs text-rose-200">{invoice.clientCompanyName} · due {invoice.dueDate}</p>
                      </div>
                      <MoneyValue amountMinor={invoice.outstandingMinor} currency={invoice.currencyLabel} />
                    </div>
                  {/each}
                </div>
              {/if}
            </SectionCard>

            <SectionCard title="Recent payments" eyebrow="Collections" description="Latest recorded payments across the ledger.">
              {#if dashboard.recentPayments.length === 0}
                <p class="text-sm text-slate-400">No payments recorded yet.</p>
              {:else}
                <div class="space-y-3">
                  {#each dashboard.recentPayments as payment}
                    <div class="panel-soft flex items-center justify-between gap-4 p-4">
                      <div>
                        <p class="font-medium text-white">
                          {payment.invoiceNumber ?? 'Invoice'} · {payment.clientCompanyName ?? 'Client'}
                        </p>
                        <p class="text-xs text-slate-400">
                          {payment.paymentDate} · {payment.paymentSource}
                          {#if payment.transactionReferenceId}
                            · {payment.transactionReferenceId}
                          {/if}
                        </p>
                      </div>
                      <div class="text-right">
                        <MoneyValue amountMinor={payment.amountMinor} currency={payment.currencyLabel} />
                        <p class="mt-1 text-xs text-slate-500">Source</p>
                        <MoneyValue amountMinor={payment.convertedAmountMinor ?? payment.amountMinor} currency={payment.reportingCurrencyLabel} muted />
                        <p class="mt-1 text-xs text-slate-500">Reporting</p>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </SectionCard>
          </div>
        </div>

        <SectionCard title="Currency totals" eyebrow="Reporting" description="Source totals by payment currency, invoice totals, and reporting income snapshots.">
          <div class="grid gap-4 md:grid-cols-3">
            <div class="panel-soft p-4">
              <p class="label">Invoiced by currency</p>
              {#if dashboard.totalInvoicedByCurrency.length === 0}
                <p class="mt-3 text-sm text-slate-500">No invoice totals yet.</p>
              {:else}
                <div class="mt-3 space-y-2">
                  {#each dashboard.totalInvoicedByCurrency as item}
                    <div class="flex items-center justify-between gap-4 text-sm">
                      <span class="text-slate-300">{item.currencyLabel}</span>
                      <MoneyValue amountMinor={item.amountMinor} currency={item.currencyLabel} />
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
            <div class="panel-soft p-4">
              <p class="label">Paid by source currency</p>
              {#if dashboard.totalPaidByCurrency.length === 0}
                <p class="mt-3 text-sm text-slate-500">No payment totals yet.</p>
              {:else}
                <div class="mt-3 space-y-2">
                  {#each dashboard.totalPaidByCurrency as item}
                    <div class="flex items-center justify-between gap-4 text-sm">
                      <span class="text-slate-300">{item.currencyLabel}</span>
                      <MoneyValue amountMinor={item.amountMinor} currency={item.currencyLabel} />
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
            <div class="panel-soft p-4">
              <p class="label">Reporting income</p>
              {#if dashboard.convertedIncomeByCurrency.length === 0}
                <p class="mt-3 text-sm text-slate-500">No conversion snapshots yet.</p>
              {:else}
                <div class="mt-3 space-y-2">
                  {#each dashboard.convertedIncomeByCurrency as item}
                    <div class="flex items-center justify-between gap-4 text-sm">
                      <span class="text-slate-300">{item.currencyLabel}</span>
                      <MoneyValue amountMinor={item.amountMinor} currency={item.currencyLabel} />
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        </SectionCard>

        <SectionCard title="CSV export" eyebrow="Local files" description="Write report exports to a local path you choose.">
          <div class="grid gap-4 md:grid-cols-2">
            <label class="space-y-2">
              <span class="label">Invoices CSV path</span>
              <input class="input-base" bind:value={invoiceCsvPath} placeholder="E:\\exports\\invoices.csv" />
            </label>
            <label class="space-y-2">
              <span class="label">Payments CSV path</span>
              <input class="input-base" bind:value={paymentCsvPath} placeholder="E:\\exports\\payments.csv" />
            </label>
          </div>
          <div class="mt-4 flex flex-wrap gap-3">
            <button class="button-primary" disabled={exporting} on:click={exportInvoices} type="button">Export invoices CSV</button>
            <button class="button-secondary" disabled={exporting} on:click={exportPayments} type="button">Export payments CSV</button>
          </div>
        </SectionCard>
      {/if}
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
