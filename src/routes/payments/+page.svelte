<script lang="ts">
  import { onMount } from 'svelte';

  import {
    deletePayment,
    getAppState,
    getPayment,
    listInvoices,
    listClients,
    listPayments,
    recordPayment,
    updatePayment,
  } from '$lib/api/tauri';
  import MoneyValue from '$lib/components/MoneyValue.svelte';
  import SectionCard from '$lib/components/SectionCard.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import { moneyFormatPreference } from '$lib/stores/settings';
  import type {
    Client,
    InvoiceSummary,
    Payment,
    PaymentInput,
    PaymentSource,
  } from '$lib/types/domain';
  import { blankToNull, formatMinorAmount, parseMinorAmount, todayIsoDate } from '$lib/utils/money';

  type PaymentFormState = {
    invoiceId: string;
    amountMajor: string;
    currencyLabel: string;
    convertedAmountMajor: string;
    conversionRate: string;
    paymentDate: string;
    paymentSource: PaymentSource;
    transactionReferenceId: string;
    notes: string;
  };

  function formatMoney(amountMinor: number) {
    return formatMinorAmount(amountMinor, $moneyFormatPreference);
  }

  const emptyPaymentForm = (): PaymentFormState => ({
    invoiceId: '',
    amountMajor: formatMoney(0),
    currencyLabel: '',
    convertedAmountMajor: '',
    conversionRate: '',
    paymentDate: todayIsoDate(),
    paymentSource: 'Bank Transfer',
    transactionReferenceId: '',
    notes: '',
  });

  let payments: Payment[] = [];
  let invoices: InvoiceSummary[] = [];
  let clients: Client[] = [];
  let selectedPaymentId: number | null = null;
  let selectedPayment: Payment | null = null;
  let viewMode: 'list' | 'view' | 'edit' = 'list';
  let form: PaymentFormState = emptyPaymentForm();
  let reportingCurrencyLabel = 'CAD';
  let filters = {
    search: '',
    invoiceId: '',
    clientId: '',
    currencyLabel: '',
    fromDate: '',
    toDate: '',
  };
  let loading = true;
  let saving = false;
  let error: string | null = null;
  let notice: string | null = null;

  $: selectedPaymentInvoice = selectedPayment ? invoices.find((invoice) => invoice.id === selectedPayment.invoiceId) ?? null : null;
  $: isReadOnly = viewMode === 'view';
  $: payableInvoices = invoices.filter(
    (invoice) =>
      invoice.status !== 'Draft' &&
      invoice.status !== 'Cancelled' &&
      invoice.outstandingMinor > 0,
  );

  function currentPaymentFilters() {
    return {
      search: blankToNull(filters.search),
      invoiceId: filters.invoiceId ? Number.parseInt(filters.invoiceId, 10) : null,
      clientId: filters.clientId ? Number.parseInt(filters.clientId, 10) : null,
      currencyLabel: blankToNull(filters.currencyLabel),
      fromDate: blankToNull(filters.fromDate),
      toDate: blankToNull(filters.toDate),
    };
  }

  async function refreshData() {
    loading = true;
    error = null;
    try {
      const [nextPayments, nextInvoices, nextClients, nextState] = await Promise.all([
        listPayments(currentPaymentFilters()),
        listInvoices({}),
        listClients(null, true),
        getAppState(),
      ]);
      payments = nextPayments;
      invoices = nextInvoices;
      clients = nextClients;
      reportingCurrencyLabel = nextState.reportingCurrencyLabel || nextState.settings.reporting_currency_label || nextState.settings.default_currency_label || 'CAD';
      if (selectedPaymentId !== null) {
        selectedPayment = await getPayment(selectedPaymentId);
        form = paymentToForm(selectedPayment);
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  function paymentToForm(payment: Payment): PaymentFormState {
    return {
      invoiceId: String(payment.invoiceId),
      amountMajor: formatMoney(payment.amountMinor),
      currencyLabel: payment.currencyLabel,
      convertedAmountMajor: payment.convertedAmountMinor !== null ? formatMoney(payment.convertedAmountMinor) : '',
      conversionRate: payment.conversionRate ?? '',
      paymentDate: payment.paymentDate,
      paymentSource: payment.paymentSource,
      transactionReferenceId: payment.transactionReferenceId ?? '',
      notes: payment.notes ?? '',
    };
  }

  function startNewPayment() {
    selectedPaymentId = null;
    selectedPayment = null;
    form = emptyPaymentForm();
    notice = null;
    error = null;
    viewMode = 'edit';
    if (payableInvoices.length > 0) {
      const nonDraft = payableInvoices[0];
      if (nonDraft) {
        form.invoiceId = String(nonDraft.id);
        form.currencyLabel = nonDraft.currencyLabel;
      }
    }
  }

  function backToPaymentList() {
    viewMode = 'list';
  }

  function handleInvoiceChange() {
    const selected = payableInvoices.find((invoice) => invoice.id === Number.parseInt(form.invoiceId, 10));
    if (selected && !form.currencyLabel) {
      form.currencyLabel = selected.currencyLabel;
    }
  }

  onMount(async () => {
    await refreshData();
    if (payableInvoices.length > 0) {
      const candidate = payableInvoices[0];
      form.invoiceId = String(candidate.id);
      form.currencyLabel = candidate.currencyLabel;
    }
  });

  function buildPaymentInput(): PaymentInput {
    if (!form.invoiceId) {
      throw new Error('Invoice is required');
    }

    const selectedInvoice = invoices.find((invoice) => invoice.id === Number.parseInt(form.invoiceId, 10));
    if (selectedInvoice && form.currencyLabel.trim() !== selectedInvoice.currencyLabel) {
      throw new Error('Source currency must match the invoice currency. Use the reporting amount field for bookkeeping.');
    }

    if (!form.currencyLabel.trim()) {
      throw new Error('Currency label is required');
    }

    return {
      invoiceId: Number.parseInt(form.invoiceId, 10),
      amountMinor: parseMinorAmount(form.amountMajor),
      currencyLabel: form.currencyLabel.trim(),
      reportingCurrencyLabel: reportingCurrencyLabel.trim() || 'CAD',
      convertedAmountMinor: blankToNull(form.convertedAmountMajor) ? parseMinorAmount(form.convertedAmountMajor) : null,
      conversionRate: blankToNull(form.conversionRate),
      paymentDate: form.paymentDate,
      paymentSource: form.paymentSource,
      transactionReferenceId: blankToNull(form.transactionReferenceId),
      notes: blankToNull(form.notes),
    };
  }

  async function savePayment() {
    if (isReadOnly) {
      return;
    }

    saving = true;
    error = null;
    notice = null;

    try {
      const input = buildPaymentInput();
      const saved = selectedPaymentId === null
        ? await recordPayment(input)
        : await updatePayment(selectedPaymentId, input);

      selectedPaymentId = saved.id;
      selectedPayment = saved;
      form = paymentToForm(saved);
      notice = `Payment recorded locally for invoice ${saved.invoiceNumber ?? saved.invoiceId}.`;
      await refreshData();
      selectedPaymentId = saved.id;
      selectedPayment = await getPayment(saved.id);
      form = paymentToForm(selectedPayment);
      viewMode = 'view';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function loadPayment(paymentId: number, nextMode: 'view' | 'edit' = 'view') {
    selectedPaymentId = paymentId;
    notice = null;
    error = null;
    viewMode = nextMode;
    try {
      selectedPayment = await getPayment(paymentId);
      form = paymentToForm(selectedPayment);
    } catch (cause) {
      selectedPaymentId = null;
      selectedPayment = null;
      form = emptyPaymentForm();
      viewMode = 'list';
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function viewSelectedPayment() {
    if (selectedPaymentId !== null) {
      await loadPayment(selectedPaymentId, 'view');
    }
  }

  async function editSelectedPayment() {
    if (selectedPaymentId !== null) {
      await loadPayment(selectedPaymentId, 'edit');
    }
  }

  async function deleteSelectedPayment() {
    if (selectedPaymentId === null) {
      return;
    }

    if (!window.confirm('Delete this payment? The invoice totals will be recomputed locally.')) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      await deletePayment(selectedPaymentId);
      notice = 'Payment deleted locally.';
      selectedPaymentId = null;
      selectedPayment = null;
      form = emptyPaymentForm();
      await refreshData();
      viewMode = 'list';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function deletePaymentFromList(paymentId: number) {
    if (!window.confirm('Delete this payment? The invoice totals will be recomputed locally.')) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      await deletePayment(paymentId);
      notice = 'Payment deleted locally.';
      if (selectedPaymentId === paymentId) {
        selectedPaymentId = null;
        selectedPayment = null;
        form = emptyPaymentForm();
        viewMode = 'list';
      }
      await refreshData();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

</script>

<div class="space-y-6">
  <SectionCard
    title="Payments"
    eyebrow="Collections"
    description="Record, view, edit, and delete local payment entries. The Rust backend recomputes invoice status after every change."
  >
    <svelte:fragment slot="actions">
      {#if viewMode === 'view'}
        <button class="button-secondary" on:click={editSelectedPayment} type="button">Edit</button>
        <button class="button-secondary" on:click={backToPaymentList} type="button">Back to list</button>
      {:else if viewMode === 'edit'}
        {#if selectedPaymentId !== null}
          <button class="button-secondary" on:click={viewSelectedPayment} type="button">View</button>
        {/if}
        <button class="button-secondary" on:click={backToPaymentList} type="button">Back to list</button>
      {:else}
        <button class="button-secondary" on:click={startNewPayment} type="button">New payment</button>
      {/if}
      <button class="button-secondary" disabled={saving} on:click={refreshData} type="button">Refresh</button>
    </svelte:fragment>

    {#if viewMode === 'list'}
      <div class="space-y-4">
        <div class="panel-soft p-4">
          <div class="grid gap-4 lg:grid-cols-3 xl:grid-cols-6">
            <label class="space-y-2 xl:col-span-2">
              <span class="label">Search</span>
              <input class="input-base" bind:value={filters.search} placeholder="Invoice number, client, reference" />
            </label>
            <label class="space-y-2">
              <span class="label">Invoice</span>
              <select class="input-base" bind:value={filters.invoiceId}>
                <option value="">All invoices</option>
                {#each invoices as invoice}
                  <option value={String(invoice.id)}>{invoice.invoiceNumber}</option>
                {/each}
              </select>
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
            <label class="space-y-2">
              <span class="label">From</span>
              <input class="input-base" bind:value={filters.fromDate} type="date" />
            </label>
            <label class="space-y-2">
              <span class="label">To</span>
              <input class="input-base" bind:value={filters.toDate} type="date" />
            </label>
          </div>

          <div class="mt-4 flex flex-wrap items-center gap-3">
            <button class="button-primary" disabled={saving} on:click={refreshData} type="button">Apply filters</button>
            <p class="text-xs text-slate-500">{payments.length} payments match the current filters.</p>
          </div>
        </div>

        <div class="overflow-hidden rounded-2xl bg-white/[0.02]">
          <div class="overflow-auto">
            <table class="min-w-full divide-y divide-white/10 text-sm">
              <thead class="bg-white/[0.03] text-left text-xs uppercase tracking-[0.16em] text-slate-400">
                <tr>
                  <th class="px-4 py-3">Date</th>
                  <th class="px-4 py-3">Invoice</th>
                  <th class="px-4 py-3">Source</th>
                  <th class="px-4 py-3 text-right">Amounts</th>
                  <th class="px-4 py-3">Actions</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-white/10">
                {#if loading}
                  <tr>
                    <td class="px-4 py-6 text-slate-400" colspan="5">Loading payments from local SQLite...</td>
                  </tr>
                {:else if payments.length === 0}
                  <tr>
                    <td class="px-4 py-6 text-slate-400" colspan="5">No payments match the current filters.</td>
                  </tr>
                {:else}
                  {#each payments as payment}
                    <tr class={`bg-white/[0.02] transition hover:bg-white/[0.04] ${selectedPaymentId === payment.id ? 'bg-accent-500/10' : ''}`}>
                      <td class="px-4 py-3 text-slate-300">{payment.paymentDate}</td>
                      <td class="px-4 py-3 text-white">{payment.invoiceNumber ?? `Invoice #${payment.invoiceId}`}</td>
                      <td class="px-4 py-3 text-slate-300">{payment.paymentSource}</td>
                      <td class="px-4 py-3 text-right">
                        <div class="space-y-1">
                          <MoneyValue amountMinor={payment.amountMinor} currency={payment.currencyLabel} />
                          <p class="text-[11px] uppercase tracking-[0.18em] text-slate-500">Source</p>
                          <MoneyValue
                            amountMinor={payment.convertedAmountMinor ?? payment.amountMinor}
                            currency={payment.reportingCurrencyLabel}
                            muted
                          />
                          <p class="text-[11px] uppercase tracking-[0.18em] text-slate-500">Reporting</p>
                        </div>
                      </td>
                      <td class="px-4 py-3">
                        <div class="flex flex-wrap gap-2">
                          <button class="button-secondary" on:click={() => loadPayment(payment.id, 'view')} type="button">View</button>
                          <button class="button-secondary" disabled={saving} on:click={() => deletePaymentFromList(payment.id)} type="button">
                            Delete
                          </button>
                        </div>
                      </td>
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    {:else}
      <div class="max-w-5xl space-y-4">
        <div class="panel-soft p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="label">{isReadOnly ? 'Viewing payment' : 'Editing payment'}</p>
              <h3 class="mt-1 text-lg font-semibold text-white">
                {selectedPayment?.paymentDate ?? 'New payment'}
              </h3>
            </div>
            {#if selectedPayment}
              <StatusBadge status={selectedPaymentInvoice?.status ?? 'Sent'} />
            {/if}
          </div>

          {#if selectedPayment}
            <div class="mt-4 grid gap-3 text-sm text-slate-300 md:grid-cols-2">
              <p><span class="text-slate-500">Invoice:</span> {selectedPayment.invoiceNumber ?? `#${selectedPayment.invoiceId}`}</p>
              <p><span class="text-slate-500">Client:</span> {selectedPayment.clientCompanyName ?? '—'}</p>
              <p><span class="text-slate-500">Source:</span> {selectedPayment.paymentSource}</p>
              <p><span class="text-slate-500">Reference:</span> {selectedPayment.transactionReferenceId ?? '—'}</p>
              <p><span class="text-slate-500">Source amount:</span> <MoneyValue amountMinor={selectedPayment.amountMinor} currency={selectedPayment.currencyLabel} /></p>
              <p><span class="text-slate-500">Reporting currency:</span> {selectedPayment.reportingCurrencyLabel}</p>
              <p><span class="text-slate-500">Reporting amount:</span> <MoneyValue amountMinor={selectedPayment.convertedAmountMinor ?? selectedPayment.amountMinor} currency={selectedPayment.reportingCurrencyLabel} /></p>
              <p><span class="text-slate-500">Conversion rate:</span> {selectedPayment.conversionRate ?? '—'}</p>
              <p><span class="text-slate-500">Invoice status:</span> {selectedPaymentInvoice?.status ?? 'Unknown'}</p>
            </div>
          {:else}
            <p class="mt-4 text-sm text-slate-400">Create a payment or select one from the list to inspect its bookkeeping snapshot.</p>
          {/if}
        </div>

        <form class="panel-soft space-y-4 p-4" on:submit|preventDefault={savePayment}>
          <div class="grid gap-4 md:grid-cols-2">
            <label class="space-y-2 md:col-span-2">
              <span class="label">Invoice</span>
              <select class="input-base" bind:value={form.invoiceId} disabled={isReadOnly} on:change={handleInvoiceChange}>
                <option value="">Select an invoice</option>
                {#each payableInvoices as invoice}
                  <option value={String(invoice.id)}>
                    {invoice.invoiceNumber} · {invoice.clientCompanyName} · {invoice.status}
                  </option>
                {/each}
              </select>
              <p class="text-xs text-slate-500">
                Draft, cancelled, and fully settled invoices are excluded because payments can only be attached to invoices with an outstanding balance. The source currency must match the invoice currency; the reporting currency is stored separately for bookkeeping and can be changed in Settings.
              </p>
            </label>
            <label class="space-y-2">
              <span class="label">Source amount</span>
              <input class="input-base" bind:value={form.amountMajor} disabled={isReadOnly} placeholder="0.00" />
            </label>
            <label class="space-y-2">
              <span class="label">Source currency label</span>
              <input class="input-base" bind:value={form.currencyLabel} disabled={isReadOnly} placeholder="USD, Credits, Gold" />
            </label>
            <label class="space-y-2">
              <span class="label">Reporting amount ({reportingCurrencyLabel})</span>
              <input class="input-base" bind:value={form.convertedAmountMajor} disabled={isReadOnly} placeholder="Leave blank to auto-calc" />
            </label>
            <label class="space-y-2">
              <span class="label">Source → reporting rate</span>
              <input class="input-base" bind:value={form.conversionRate} disabled={isReadOnly} placeholder="1.00" />
            </label>
            <label class="space-y-2">
              <span class="label">Payment date</span>
              <input class="input-base" bind:value={form.paymentDate} disabled={isReadOnly} type="date" />
            </label>
            <label class="space-y-2">
              <span class="label">Payment source</span>
              <select class="input-base" bind:value={form.paymentSource} disabled={isReadOnly}>
                <option>Wise Business</option>
                <option>Bank Transfer</option>
                <option>PayPal</option>
                <option>Other</option>
              </select>
            </label>
            <label class="space-y-2 md:col-span-2">
              <span class="label">Transaction / reference ID</span>
              <input class="input-base" bind:value={form.transactionReferenceId} disabled={isReadOnly} placeholder="Reference or bank note" />
            </label>
            <label class="space-y-2 md:col-span-2">
              <span class="label">Notes</span>
              <textarea class="input-base min-h-[90px] resize-y" bind:value={form.notes} disabled={isReadOnly}></textarea>
            </label>
          </div>

          <div class="flex flex-wrap items-center gap-3">
            {#if isReadOnly}
              <button class="button-primary" disabled={saving || selectedPaymentId === null} on:click={editSelectedPayment} type="button">
                Edit
              </button>
            {:else}
              <button class="button-primary" disabled={saving || payableInvoices.length === 0} type="submit">
                {selectedPaymentId === null ? 'Record payment' : 'Save payment'}
              </button>
              {#if selectedPaymentId !== null}
                <button class="button-secondary" disabled={saving} on:click={viewSelectedPayment} type="button">
                  View
                </button>
              {/if}
            {/if}
            <button class="button-secondary" disabled={saving || selectedPaymentId === null} on:click={deleteSelectedPayment} type="button">
              Delete selected
            </button>
          </div>
        </form>

        {#if selectedPaymentInvoice}
          <div class="panel-soft p-4">
            <p class="label">Linked invoice</p>
            <div class="mt-3 grid gap-3 text-sm text-slate-300 md:grid-cols-2">
              <p><span class="text-slate-500">Invoice number:</span> {selectedPaymentInvoice.invoiceNumber}</p>
              <p><span class="text-slate-500">Status:</span> <StatusBadge status={selectedPaymentInvoice.status} /></p>
              <p><span class="text-slate-500">Outstanding:</span> <MoneyValue amountMinor={selectedPaymentInvoice.outstandingMinor} currency={selectedPaymentInvoice.currencyLabel} /></p>
              <p><span class="text-slate-500">Total:</span> <MoneyValue amountMinor={selectedPaymentInvoice.totalMinor} currency={selectedPaymentInvoice.currencyLabel} /></p>
            </div>
          </div>
        {/if}
      </div>
    {/if}
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
