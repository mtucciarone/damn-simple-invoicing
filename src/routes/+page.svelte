<script lang="ts">
  import { onMount } from 'svelte';

  import { getDashboardSummary } from '$lib/api/tauri';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import MetricCard from '$lib/components/MetricCard.svelte';
  import MoneyValue from '$lib/components/MoneyValue.svelte';
  import SectionCard from '$lib/components/SectionCard.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import { moneyFormatPreference } from '$lib/stores/settings';
  import type { DashboardSummary } from '$lib/types/domain';
  import { formatMinorAmount } from '$lib/utils/money';

  let dashboard: DashboardSummary | null = null;
  let loading = true;
  let error: string | null = null;

  onMount(async () => {
    try {
      dashboard = await getDashboardSummary({});
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  });

  const formatCount = (value: number) => new Intl.NumberFormat().format(value);
  const formatMoney = (amountMinor: number) => formatMinorAmount(amountMinor, $moneyFormatPreference);
</script>

<div class="space-y-6">
  <section class="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
    <MetricCard
      label="Total invoiced"
      value={dashboard ? formatMoney(dashboard.totalInvoicedMinor) : '—'}
      detail="All non-draft invoices"
      tone="accent"
    />
    <MetricCard
      label="Source paid"
      value={dashboard ? formatMoney(dashboard.totalPaidMinor) : '—'}
      detail="Raw payment amounts in source currencies"
    />
    <MetricCard
      label="Tax total"
      value={dashboard ? `${formatMoney(dashboard.reportedIncomeMinor)} ${dashboard.reportingCurrencyLabel}` : '—'}
      detail="Payments converted to the reporting currency"
      tone="warm"
    />
    <MetricCard
      label="Outstanding"
      value={dashboard ? formatMoney(dashboard.outstandingBalanceMinor) : '—'}
      detail="Open invoice balance"
      tone="warm"
    />
    <MetricCard
      label="Invoice count"
      value={dashboard ? formatCount(dashboard.invoiceCount) : '—'}
      detail={dashboard ? `${dashboard.overdueInvoiceCount} overdue` : 'No data yet'}
    />
  </section>

  {#if loading}
    <SectionCard title="Dashboard" eyebrow="Loading" description="Reading local data from SQLite...">
      <p class="text-sm text-slate-400">The Rust backend is loading your local invoice store.</p>
    </SectionCard>
  {:else if error}
    <EmptyState
      title="Dashboard unavailable"
      description={`The app could not load dashboard data from the local SQLite database. ${error}`}
      actionLabel="Create a business profile"
      href="/business"
    />
  {:else if !dashboard}
    <EmptyState
      title="No dashboard data"
      description="Create a business profile, add a client, then draft your first invoice to see reporting here."
      actionLabel="Set up business"
      href="/business"
    />
  {:else}
    <div class="grid gap-6 xl:grid-cols-[1.35fr_0.95fr]">
      <SectionCard
        title="Recent invoices"
        eyebrow="Activity"
        description="Newest invoices from your local SQLite ledger."
      >
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
                    <td class="px-4 py-3 text-right text-slate-100">
                      <MoneyValue amountMinor={invoice.totalMinor} currency={invoice.currencyLabel} />
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </SectionCard>

      <div class="space-y-6">
        <SectionCard
          title="Overdue invoices"
          eyebrow="Risk"
          description="Invoices that need attention now."
        >
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

        <SectionCard
          title="Recent payments"
          eyebrow="Collections"
          description="Latest recorded payments across the ledger."
        >
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

    <SectionCard
      title="Currency totals"
      eyebrow="Reporting"
      description="Source totals by payment currency, invoice totals, and reporting income snapshots."
    >
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
  {/if}
</div>
