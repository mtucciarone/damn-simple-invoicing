<script lang="ts">
  import { onMount } from 'svelte';

  import {
    createInvoice,
    deleteInvoice,
    deletePayment,
    duplicateInvoice,
    exportInvoiceHtmlOnly,
    exportInvoicePdf,
    finalizeInvoice,
    getActiveBusiness,
    getInvoice,
    openLocalPath,
    listBusinesses,
    listClients,
    listInvoices,
    openInvoicePdf,
    updateInvoice,
  } from '$lib/api/tauri';
  import MoneyValue from '$lib/components/MoneyValue.svelte';
  import SectionCard from '$lib/components/SectionCard.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import type {
    BusinessProfile,
    Client,
    InvoiceDetail,
    InvoiceFilters,
    InvoiceFormInput,
    InvoiceLineItemInput,
    InvoiceStatus,
    InvoiceSummary,
  } from '$lib/types/domain';
  import { blankToNull, formatMinorAmount, parseMinorAmount, todayIsoDate } from '$lib/utils/money';
  import { parentDirectory } from '$lib/utils/path';

  type LineItemForm = {
    id: number | null;
    description: string;
    quantity: string;
    rateMajor: string;
  };

  type InvoiceFormState = {
    businessId: string;
    clientId: string;
    invoiceNumber: string;
    issueDate: string;
    dueDate: string;
    currencyLabel: string;
    notes: string;
    paymentTerms: string;
    lineItems: LineItemForm[];
  };

  const blankLineItem = (): LineItemForm => ({
    id: null,
    description: '',
    quantity: '1',
    rateMajor: '0.00',
  });

  const addDays = (days: number) => {
    const date = new Date();
    date.setDate(date.getDate() + days);
    return todayIsoDate(date);
  };

  const emptyInvoiceForm = (businessId: number | null = null): InvoiceFormState => ({
    businessId: businessId ? String(businessId) : '',
    clientId: '',
    invoiceNumber: '',
    issueDate: todayIsoDate(),
    dueDate: addDays(14),
    currencyLabel: '',
    notes: '',
    paymentTerms: '',
    lineItems: [blankLineItem()],
  });

  const invoiceDetailToForm = (detail: InvoiceDetail): InvoiceFormState => ({
    businessId: String(detail.invoice.businessId),
    clientId: String(detail.invoice.clientId),
    invoiceNumber: detail.invoice.invoiceNumber,
    issueDate: detail.invoice.issueDate,
    dueDate: detail.invoice.dueDate,
    currencyLabel: detail.invoice.currencyLabel,
    notes: detail.invoice.notes ?? '',
    paymentTerms: detail.invoice.paymentTerms ?? '',
    lineItems: detail.lineItems.length > 0
      ? detail.lineItems.map((lineItem) => ({
          id: lineItem.id,
          description: lineItem.description,
          quantity: lineItem.quantity,
          rateMajor: formatMinorAmount(lineItem.rateMinor),
        }))
      : [blankLineItem()],
  });

  let invoices: InvoiceSummary[] = [];
  let selectedInvoiceId: number | null = null;
  let selectedInvoiceDetail: InvoiceDetail | null = null;
  let viewMode: 'list' | 'view' | 'edit' = 'list';
  let clients: Client[] = [];
  let businesses: BusinessProfile[] = [];
  let activeBusiness: BusinessProfile | null = null;
  let filters = {
    search: '',
    clientId: '',
    currencyLabel: '',
    status: '',
    fromDate: '',
    toDate: '',
  };
  let form: InvoiceFormState = emptyInvoiceForm();
  let pdfOutputDir = '';
  let lastExportedPdfPath: string | null = null;
  let lastExportedHtmlPath: string | null = null;
  let loading = true;
  let saving = false;
  let error: string | null = null;
  let notice: string | null = null;

  $: isDraftInvoice = selectedInvoiceDetail?.invoice?.status === 'Draft' || selectedInvoiceId === null;
  $: isReadOnly = viewMode === 'view';
  $: formLocked = isReadOnly || (selectedInvoiceId !== null && !isDraftInvoice);

  function currentInvoiceFilters(): InvoiceFilters {
    return {
      search: blankToNull(filters.search),
      clientId: filters.clientId ? Number.parseInt(filters.clientId, 10) : null,
      currencyLabel: blankToNull(filters.currencyLabel),
      status: (filters.status || null) as InvoiceStatus | null,
      fromDate: blankToNull(filters.fromDate),
      toDate: blankToNull(filters.toDate),
    };
  }

  function clearExportArtifacts() {
    lastExportedPdfPath = null;
    lastExportedHtmlPath = null;
  }

  async function refreshLookupData() {
    const [nextClients, nextBusinesses, nextActiveBusiness] = await Promise.all([
      listClients(null, true),
      listBusinesses(true),
      getActiveBusiness(),
    ]);

    clients = nextClients;
    businesses = nextBusinesses;
    activeBusiness = nextActiveBusiness ?? businesses.find((business) => business.isActive) ?? null;

    if (selectedInvoiceId === null && !form.businessId) {
      form.businessId = activeBusiness ? String(activeBusiness.id) : '';
    }
  }

  async function refreshInvoices() {
    loading = true;
    error = null;
    try {
      await refreshLookupData();
      invoices = await listInvoices(currentInvoiceFilters());
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  async function loadInvoiceDetail(invoiceId: number, nextMode: 'view' | 'edit' = 'view') {
    selectedInvoiceId = invoiceId;
    error = null;
    notice = null;
    try {
      selectedInvoiceDetail = await getInvoice(invoiceId);
      form = invoiceDetailToForm(selectedInvoiceDetail);
      pdfOutputDir = '';
      clearExportArtifacts();
      viewMode = nextMode;
    } catch (cause) {
      selectedInvoiceDetail = null;
      selectedInvoiceId = null;
      form = emptyInvoiceForm(activeBusiness?.id ?? null);
      pdfOutputDir = '';
      clearExportArtifacts();
      viewMode = 'list';
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function startNewInvoice() {
    selectedInvoiceId = null;
    selectedInvoiceDetail = null;
    form = emptyInvoiceForm(activeBusiness?.id ?? null);
    pdfOutputDir = '';
    clearExportArtifacts();
    notice = null;
    error = null;
    viewMode = 'edit';
  }

  function backToInvoiceList() {
    viewMode = 'list';
  }

  async function viewSelectedInvoice() {
    if (selectedInvoiceId !== null) {
      await loadInvoiceDetail(selectedInvoiceId, 'view');
    }
  }

  async function editSelectedInvoice() {
    if (selectedInvoiceId !== null && isDraftInvoice) {
      await loadInvoiceDetail(selectedInvoiceId, 'edit');
    }
  }

  onMount(async () => {
    await refreshInvoices();
    if (activeBusiness) {
      form.businessId = String(activeBusiness.id);
    }
  });

  function addLineItem() {
    if (formLocked) {
      return;
    }

    form.lineItems = [...form.lineItems, blankLineItem()];
  }

  function removeLineItem(index: number) {
    if (formLocked) {
      return;
    }

    const next = form.lineItems.filter((_, itemIndex) => itemIndex !== index);
    form.lineItems = next.length > 0 ? next : [blankLineItem()];
  }

  function moveLineItem(index: number, direction: -1 | 1) {
    if (formLocked) {
      return;
    }

    const target = index + direction;
    if (target < 0 || target >= form.lineItems.length) {
      return;
    }

    const next = [...form.lineItems];
    [next[index], next[target]] = [next[target], next[index]];
    form.lineItems = next;
  }

  function buildInvoiceInput(): InvoiceFormInput {
    if (!form.clientId) {
      throw new Error('Client is required');
    }

    if (!form.currencyLabel.trim()) {
      throw new Error('Currency label is required');
    }

    return {
      businessId: form.businessId ? Number.parseInt(form.businessId, 10) : null,
      clientId: Number.parseInt(form.clientId, 10),
      invoiceNumber: blankToNull(form.invoiceNumber),
      issueDate: form.issueDate,
      dueDate: form.dueDate,
      currencyLabel: form.currencyLabel.trim(),
      notes: blankToNull(form.notes),
      paymentTerms: blankToNull(form.paymentTerms),
      lineItems: form.lineItems.map<InvoiceLineItemInput>((lineItem) => ({
        description: lineItem.description.trim(),
        quantity: lineItem.quantity.trim(),
        rateMinor: parseMinorAmount(lineItem.rateMajor),
      })),
    };
  }

  async function saveInvoice() {
    if (isReadOnly) {
      return;
    }

    if (formLocked) {
      error = 'Selected invoice is finalized. Duplicate it to create an editable draft.';
      return;
    }

    saving = true;
    error = null;
    notice = null;

    try {
      const input = buildInvoiceInput();
      const saved = selectedInvoiceId === null
        ? await createInvoice(input)
        : await updateInvoice(selectedInvoiceId, input);

      selectedInvoiceId = saved.id;
      selectedInvoiceDetail = saved;
      form = invoiceDetailToForm(saved);
      clearExportArtifacts();
      notice = `Invoice ${saved.invoice.invoiceNumber} saved locally.`;
      await refreshInvoices();
      selectedInvoiceId = saved.id;
      selectedInvoiceDetail = await getInvoice(saved.id);
      form = invoiceDetailToForm(selectedInvoiceDetail);
      viewMode = 'view';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function duplicateSelectedInvoice() {
    if (selectedInvoiceId === null) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      const saved = await duplicateInvoice(selectedInvoiceId);
      selectedInvoiceId = saved.id;
      selectedInvoiceDetail = saved;
      form = invoiceDetailToForm(saved);
      notice = `Invoice ${saved.invoice.invoiceNumber} duplicated as a new draft.`;
      await refreshInvoices();
      selectedInvoiceDetail = await getInvoice(saved.id);
      form = invoiceDetailToForm(selectedInvoiceDetail);
      viewMode = 'edit';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function finalizeSelectedInvoice() {
    if (selectedInvoiceId === null) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      const saved = await finalizeInvoice(selectedInvoiceId);
      selectedInvoiceDetail = saved;
      form = invoiceDetailToForm(saved);
      clearExportArtifacts();
      notice = `Invoice ${saved.invoice.invoiceNumber} finalized locally.`;
      await refreshInvoices();
      selectedInvoiceDetail = await getInvoice(saved.invoice.id);
      form = invoiceDetailToForm(selectedInvoiceDetail);
      viewMode = 'view';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function deleteSelectedInvoice() {
    if (selectedInvoiceId === null) {
      return;
    }

    const paymentCount = selectedInvoiceDetail?.payments.length ?? 0;
    const confirmation = paymentCount > 0
      ? `Delete this invoice? It has ${paymentCount} payment(s). Delete those payments first if you want the invoice removed cleanly.`
      : 'Delete this invoice? Finalized invoices can be deleted too.';

    if (!window.confirm(confirmation)) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      await deleteInvoice(selectedInvoiceId);
      notice = 'Invoice deleted locally.';
      startNewInvoice();
      await refreshInvoices();
      viewMode = 'list';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function exportSelectedInvoicePdf(openAfter = false) {
    if (selectedInvoiceId === null) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      const result = openAfter
        ? await openInvoicePdf(selectedInvoiceId, blankToNull(pdfOutputDir))
        : await exportInvoicePdf(selectedInvoiceId, blankToNull(pdfOutputDir), false);
      lastExportedPdfPath = result.pdfPath;
      lastExportedHtmlPath = result.htmlPath;
      notice = openAfter
        ? `PDF written to ${result.pdfPath} and opened locally.`
        : `PDF written to ${result.pdfPath}`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function exportSelectedInvoiceHtml() {
    if (selectedInvoiceId === null) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      const result = await exportInvoiceHtmlOnly(selectedInvoiceId, blankToNull(pdfOutputDir));
      lastExportedPdfPath = null;
      lastExportedHtmlPath = result.htmlPath;
      notice = `HTML written to ${result.htmlPath}`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function openExportedPath(path: string) {
    saving = true;
    error = null;
    notice = null;
    try {
      await openLocalPath(path);
      notice = `Opened ${path}`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function duplicateInvoiceFromList(invoiceId: number) {
    saving = true;
    error = null;
    notice = null;
    try {
      const saved = await duplicateInvoice(invoiceId);
      await refreshInvoices();
      await loadInvoiceDetail(saved.id, 'edit');
      notice = `Invoice ${saved.invoice.invoiceNumber} duplicated as a new draft.`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function finalizeInvoiceFromList(invoiceId: number) {
    saving = true;
    error = null;
    notice = null;
    try {
      selectedInvoiceId = invoiceId;
      await finalizeSelectedInvoice();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function deleteInvoiceFromList(invoiceId: number) {
    saving = true;
    error = null;
    notice = null;
    try {
      await loadInvoiceDetail(invoiceId, 'view');
      await deleteSelectedInvoice();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function deleteInvoicePayment(paymentId: number) {
    if (!window.confirm('Delete this payment? The invoice totals will be recomputed locally.')) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      await deletePayment(paymentId);
      notice = 'Payment deleted locally.';
      if (selectedInvoiceId !== null) {
        selectedInvoiceDetail = await getInvoice(selectedInvoiceId);
        form = invoiceDetailToForm(selectedInvoiceDetail);
      }
      await refreshInvoices();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }
</script>

<div class="space-y-6">
  <SectionCard
    title="Invoices"
    eyebrow="CRUD"
    description="View invoices in read-only mode, edit drafts, export PDFs, and keep finalized invoices immutable unless you duplicate them first."
  >
    <svelte:fragment slot="actions">
      {#if viewMode === 'view'}
        {#if selectedInvoiceId !== null && isDraftInvoice}
          <button class="button-secondary" on:click={editSelectedInvoice} type="button">Edit</button>
        {:else if selectedInvoiceId !== null}
          <button class="button-secondary" on:click={duplicateSelectedInvoice} type="button">Duplicate</button>
        {/if}
        <button class="button-secondary" on:click={backToInvoiceList} type="button">Back to list</button>
      {:else if viewMode === 'edit'}
        {#if selectedInvoiceId !== null}
          <button class="button-secondary" on:click={viewSelectedInvoice} type="button">View</button>
        {/if}
        <button class="button-secondary" on:click={backToInvoiceList} type="button">Back to list</button>
      {:else}
        <button class="button-secondary" on:click={startNewInvoice} type="button">New invoice</button>
      {/if}
      <button class="button-secondary" disabled={saving} on:click={refreshInvoices} type="button">Refresh</button>
    </svelte:fragment>

    {#if viewMode === 'list'}
      <div class="space-y-4">
        <div class="panel-soft p-4">
          <div class="grid gap-4 lg:grid-cols-3 xl:grid-cols-6">
            <label class="space-y-2 xl:col-span-2">
              <span class="label">Search</span>
              <input class="input-base" bind:value={filters.search} placeholder="Invoice number, client, notes" />
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
              <span class="label">Status</span>
              <select class="input-base" bind:value={filters.status}>
                <option value="">All statuses</option>
                <option value="Draft">Draft</option>
                <option value="Sent">Sent</option>
                <option value="Partially Paid">Partially Paid</option>
                <option value="Paid">Paid</option>
                <option value="Overdue">Overdue</option>
                <option value="Cancelled">Cancelled</option>
              </select>
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
            <button class="button-primary" disabled={saving} on:click={refreshInvoices} type="button">Apply filters</button>
            <p class="text-xs text-slate-500">{invoices.length} invoices match the current filters.</p>
          </div>
        </div>

        <div class="panel-soft p-4">
            <div class="flex items-center justify-between gap-3">
              <div>
                <p class="label">Invoice list</p>
                <p class="text-sm text-slate-400">Open any invoice to swap into the read-only view first.</p>
              </div>
              <p class="text-xs text-slate-500">{invoices.length} records</p>
            </div>

          {#if loading}
            <div class="py-6 text-sm text-slate-400">Loading invoices from local SQLite...</div>
          {:else if invoices.length === 0}
            <div class="py-6 text-sm text-slate-400">No invoices match the current filters.</div>
          {:else}
            <div class="mt-4 overflow-hidden rounded-2xl bg-white/[0.02]">
              <table class="min-w-full divide-y divide-white/10 text-sm">
                <thead class="bg-white/[0.03] text-left text-xs uppercase tracking-[0.16em] text-slate-400">
                  <tr>
                    <th class="px-4 py-3">Invoice</th>
                    <th class="px-4 py-3">Client</th>
                    <th class="px-4 py-3">Status</th>
                    <th class="px-4 py-3">Issue</th>
                    <th class="px-4 py-3">Due</th>
                    <th class="px-4 py-3 text-right">Total</th>
                    <th class="px-4 py-3">Actions</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-white/10">
                  {#each invoices as invoice}
                    <tr class={`transition hover:bg-white/[0.04] ${selectedInvoiceId === invoice.id ? 'bg-accent-500/10' : 'bg-white/[0.02]'}`}>
                      <td class="px-4 py-3">
                        <p class="font-medium text-white">{invoice.invoiceNumber}</p>
                      </td>
                      <td class="px-4 py-3 text-slate-300">{invoice.clientCompanyName}</td>
                      <td class="px-4 py-3"><StatusBadge status={invoice.status} /></td>
                      <td class="px-4 py-3 text-slate-300">{invoice.issueDate}</td>
                      <td class="px-4 py-3 text-slate-300">{invoice.dueDate}</td>
                      <td class="px-4 py-3 text-right"><MoneyValue amountMinor={invoice.totalMinor} currency={invoice.currencyLabel} /></td>
                      <td class="px-4 py-3">
                        <div class="flex flex-wrap gap-2">
                          <button class="button-secondary" on:click={() => loadInvoiceDetail(invoice.id, 'view')} type="button">View</button>
                          <button class="button-secondary" disabled={saving} on:click={() => duplicateInvoiceFromList(invoice.id)} type="button">
                            Duplicate
                          </button>
                          {#if invoice.status === 'Draft'}
                            <button class="button-secondary" disabled={saving} on:click={() => finalizeInvoiceFromList(invoice.id)} type="button">
                              Finalize
                            </button>
                            <button class="button-secondary" disabled={saving} on:click={() => deleteInvoiceFromList(invoice.id)} type="button">
                              Delete
                            </button>
                          {:else}
                            <button class="button-secondary" disabled={saving} on:click={() => deleteInvoiceFromList(invoice.id)} type="button">
                              Delete
                            </button>
                          {/if}
                        </div>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <div class="max-w-5xl space-y-4">
        <div class="panel-soft p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="label">{isReadOnly ? 'Viewing invoice' : 'Editing invoice'}</p>
              <h3 class="mt-1 text-lg font-semibold text-white">
                {selectedInvoiceDetail?.invoice?.invoiceNumber ?? 'New invoice'}
              </h3>
            </div>
            {#if selectedInvoiceDetail}
              <StatusBadge status={selectedInvoiceDetail.invoice?.status ?? 'Draft'} />
            {/if}
          </div>

          {#if selectedInvoiceDetail}
            <div class="mt-4 grid gap-3 text-sm text-slate-300 md:grid-cols-2">
              <p><span class="text-slate-500">Business:</span> {selectedInvoiceDetail.invoice.businessSnapshot.businessName}</p>
              <p><span class="text-slate-500">Client:</span> {selectedInvoiceDetail.invoice.clientSnapshot.companyName}</p>
              <p><span class="text-slate-500">Issue:</span> {selectedInvoiceDetail.invoice.issueDate}</p>
              <p><span class="text-slate-500">Due:</span> {selectedInvoiceDetail.invoice.dueDate}</p>
              <p><span class="text-slate-500">Currency:</span> {selectedInvoiceDetail.invoice.currencyLabel}</p>
              <p><span class="text-slate-500">Status:</span> {selectedInvoiceDetail.invoice.status}</p>
            </div>
          {:else}
            <p class="mt-4 text-sm text-slate-400">
              Create a local draft to see invoice history, export controls, totals, and payment snapshots.
            </p>
          {/if}

          {#if isReadOnly}
            <div class="mt-4 flex flex-wrap items-center gap-2">
              <label class="flex-1 space-y-2">
                <span class="label">PDF output directory</span>
                <input class="input-base" bind:value={pdfOutputDir} placeholder="Leave blank for the local exports folder" />
              </label>
            </div>
            <div class="mt-4 flex flex-wrap gap-2">
              <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={() => exportSelectedInvoicePdf(false)} type="button">
                Export PDF
              </button>
              <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={() => exportSelectedInvoicePdf(true)} type="button">
                Export &amp; open PDF
              </button>
              <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={exportSelectedInvoiceHtml} type="button">
                Export HTML
              </button>
            </div>
            {#if lastExportedPdfPath || lastExportedHtmlPath}
              <div class="mt-4 rounded-2xl bg-white/[0.02] p-4">
                <p class="label">Last export</p>
                <div class="mt-3 space-y-2 text-sm text-slate-300">
                  {#if lastExportedPdfPath}
                    <p class="break-all"><span class="text-slate-500">PDF:</span> {lastExportedPdfPath}</p>
                  {/if}
                  {#if lastExportedHtmlPath}
                    <p class="break-all"><span class="text-slate-500">HTML:</span> {lastExportedHtmlPath}</p>
                  {/if}
                </div>
                <div class="mt-3 flex flex-wrap gap-2">
                  {#if lastExportedPdfPath}
                    <button class="button-secondary" disabled={saving} on:click={() => openExportedPath(lastExportedPdfPath)} type="button">
                      Open PDF
                    </button>
                  {/if}
                  {#if lastExportedHtmlPath}
                    <button class="button-secondary" disabled={saving} on:click={() => openExportedPath(lastExportedHtmlPath)} type="button">
                      Open HTML
                    </button>
                  {/if}
                  {#if lastExportedPdfPath || lastExportedHtmlPath}
                    <button
                      class="button-secondary"
                      disabled={saving}
                      on:click={() => openExportedPath(parentDirectory(lastExportedPdfPath ?? lastExportedHtmlPath ?? ''))}
                      type="button"
                    >
                      Open folder
                    </button>
                  {/if}
                </div>
              </div>
            {/if}
            {#if selectedInvoiceId !== null && !isDraftInvoice}
              <p class="mt-4 rounded-2xl bg-amber-400/10 px-3 py-2 text-xs text-amber-100">
                This invoice is finalized. Duplicate it to create a new editable draft.
              </p>
            {/if}
          {/if}
        </div>

        <form class="panel-soft space-y-4 p-4" on:submit|preventDefault={saveInvoice}>
          <div class="grid gap-4 md:grid-cols-2">
            <label class="space-y-2">
              <span class="label">Business</span>
              <select class="input-base" bind:value={form.businessId} disabled={formLocked}>
                <option value="">Use active business</option>
                {#each businesses as business}
                  <option value={String(business.id)}>
                    {business.businessName}{business.archivedAt ? ' (archived)' : ''}
                  </option>
                {/each}
              </select>
            </label>
            <label class="space-y-2">
              <span class="label">Client</span>
              <select class="input-base" bind:value={form.clientId} disabled={formLocked}>
                <option value="">Select a client</option>
                {#each clients as client}
                  <option value={String(client.id)}>
                    {client.companyName}{client.archivedAt ? ' (archived)' : ''}
                  </option>
                {/each}
              </select>
            </label>
            <label class="space-y-2">
              <span class="label">Invoice number</span>
              <input class="input-base" bind:value={form.invoiceNumber} disabled={formLocked} placeholder="Leave blank for auto-numbering" />
            </label>
            <label class="space-y-2">
              <span class="label">Currency label</span>
              <input class="input-base" bind:value={form.currencyLabel} disabled={formLocked} placeholder="USD, Credits, Gold" />
            </label>
            <label class="space-y-2">
              <span class="label">Issue date</span>
              <input class="input-base" bind:value={form.issueDate} disabled={formLocked} type="date" />
            </label>
            <label class="space-y-2">
              <span class="label">Due date</span>
              <input class="input-base" bind:value={form.dueDate} disabled={formLocked} type="date" />
            </label>
            <label class="space-y-2 md:col-span-2">
              <span class="label">Notes</span>
              <textarea class="input-base min-h-[88px] resize-y" bind:value={form.notes} disabled={formLocked}></textarea>
            </label>
            <label class="space-y-2 md:col-span-2">
              <span class="label">Payment terms</span>
              <textarea class="input-base min-h-[88px] resize-y" bind:value={form.paymentTerms} disabled={formLocked}></textarea>
            </label>
          </div>

          <div class="space-y-3">
            <div class="flex items-center justify-between gap-3">
              <div>
                <p class="label">Line items</p>
                <p class="text-xs text-slate-500">Amounts are entered in major units and stored as integer minor units.</p>
              </div>
              <button class="button-secondary" disabled={formLocked} on:click={addLineItem} type="button">Add line</button>
            </div>

            <div class="space-y-3">
              {#each form.lineItems as lineItem, index}
                <div class="rounded-2xl bg-white/[0.03] p-4">
                  <div class="grid gap-3 lg:grid-cols-[1.4fr_0.7fr_0.7fr_auto]">
                    <label class="space-y-2">
                      <span class="label">Description</span>
                      <input class="input-base" bind:value={lineItem.description} disabled={formLocked} placeholder="Consulting services" />
                    </label>
                    <label class="space-y-2">
                      <span class="label">Quantity / hours</span>
                      <input class="input-base" bind:value={lineItem.quantity} disabled={formLocked} placeholder="1.5" />
                    </label>
                    <label class="space-y-2">
                      <span class="label">Rate</span>
                      <input class="input-base" bind:value={lineItem.rateMajor} disabled={formLocked} placeholder="150.00" />
                    </label>
                    <div class="flex items-end gap-2">
                      <button class="button-secondary" disabled={formLocked || index === 0} on:click={() => moveLineItem(index, -1)} type="button">
                        Up
                      </button>
                      <button
                        class="button-secondary"
                        disabled={formLocked || index === form.lineItems.length - 1}
                        on:click={() => moveLineItem(index, 1)}
                        type="button"
                      >
                        Down
                      </button>
                      <button class="button-secondary" disabled={formLocked && selectedInvoiceId !== null} on:click={() => removeLineItem(index)} type="button">
                        Remove
                      </button>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </div>

          <div class="flex flex-wrap items-center gap-3">
            {#if isReadOnly}
              {#if selectedInvoiceId !== null && isDraftInvoice}
                <button class="button-primary" disabled={saving || selectedInvoiceId === null} on:click={editSelectedInvoice} type="button">
                  Edit
                </button>
              {:else if selectedInvoiceId !== null}
                <button class="button-primary" disabled={saving} on:click={duplicateSelectedInvoice} type="button">
                  Duplicate
                </button>
              {/if}
            {:else}
              <button class="button-primary" disabled={saving || formLocked} type="submit">
                {selectedInvoiceId === null ? 'Create invoice' : 'Save draft'}
              </button>
              {#if selectedInvoiceId !== null}
                <button class="button-secondary" disabled={saving} on:click={viewSelectedInvoice} type="button">
                  View
                </button>
              {/if}
            {/if}
            {#if !isReadOnly}
              <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={duplicateSelectedInvoice} type="button">
                Duplicate
              </button>
            {/if}
            <button class="button-secondary" disabled={saving || !isDraftInvoice || selectedInvoiceId === null} on:click={finalizeSelectedInvoice} type="button">
              Finalize
            </button>
            <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={deleteSelectedInvoice} type="button">
              Delete invoice
            </button>
          </div>
        </form>

        {#if isReadOnly && selectedInvoiceDetail}
          <div class="panel-soft p-4">
            <p class="label">Totals</p>
            <div class="mt-3 grid gap-3 sm:grid-cols-2">
              <div class="rounded-2xl bg-white/[0.03] p-3">
                <p class="text-xs text-slate-500">Subtotal</p>
                <p class="mt-1 text-lg font-medium text-white">
                  <MoneyValue amountMinor={selectedInvoiceDetail.invoice.subtotalMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} />
                </p>
              </div>
              <div class="rounded-2xl bg-white/[0.03] p-3">
                <p class="text-xs text-slate-500">Paid</p>
                <p class="mt-1 text-lg font-medium text-white">
                  <MoneyValue amountMinor={selectedInvoiceDetail.invoice.paidMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} />
                </p>
              </div>
              <div class="rounded-2xl bg-white/[0.03] p-3">
                <p class="text-xs text-slate-500">Outstanding</p>
                <p class="mt-1 text-lg font-medium text-white">
                  <MoneyValue amountMinor={selectedInvoiceDetail.invoice.outstandingMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} />
                </p>
              </div>
              <div class="rounded-2xl bg-white/[0.03] p-3">
                <p class="text-xs text-slate-500">Total</p>
                <p class="mt-1 text-lg font-medium text-white">
                  <MoneyValue amountMinor={selectedInvoiceDetail.invoice.totalMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} />
                </p>
              </div>
            </div>
          </div>

          <div class="panel-soft p-4">
            <p class="label">Line items</p>
            <div class="mt-3 overflow-hidden rounded-2xl bg-white/[0.02]">
              <table class="min-w-full divide-y divide-white/10 text-sm">
                <thead class="bg-white/[0.03] text-left text-xs uppercase tracking-[0.16em] text-slate-400">
                  <tr>
                    <th class="px-4 py-3">Description</th>
                    <th class="px-4 py-3">Qty</th>
                    <th class="px-4 py-3">Rate</th>
                    <th class="px-4 py-3 text-right">Total</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-white/10">
                  {#each selectedInvoiceDetail.lineItems as lineItem}
                    <tr class="bg-white/[0.02]">
                      <td class="px-4 py-3 text-white">{lineItem.description}</td>
                      <td class="px-4 py-3 text-slate-300">{lineItem.quantity}</td>
                      <td class="px-4 py-3 text-slate-300"><MoneyValue amountMinor={lineItem.rateMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} /></td>
                      <td class="px-4 py-3 text-right text-slate-100"><MoneyValue amountMinor={lineItem.lineTotalMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} /></td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>

          <div class="panel-soft p-4">
            <p class="label">Payments</p>
            {#if selectedInvoiceDetail.payments.length === 0}
              <p class="mt-3 text-sm text-slate-400">No payments recorded yet.</p>
            {:else}
              <div class="mt-3 space-y-3">
                {#each selectedInvoiceDetail.payments as payment}
                  <div class="flex flex-wrap items-center justify-between gap-4 rounded-2xl bg-white/[0.03] px-4 py-3">
                    <div>
                      <p class="font-medium text-white">{payment.paymentDate} · {payment.paymentSource}</p>
                      <p class="text-xs text-slate-400">
                        {payment.transactionReferenceId ?? 'No reference'}
                        {#if payment.conversionRate}
                          · rate {payment.conversionRate}
                        {/if}
                      </p>
                    </div>
                    <div class="flex flex-wrap items-center gap-3">
                      <div class="text-right">
                        <MoneyValue amountMinor={payment.amountMinor} currency={payment.currencyLabel} />
                        <p class="mt-1 text-[11px] uppercase tracking-[0.18em] text-slate-500">Source</p>
                        <MoneyValue amountMinor={payment.convertedAmountMinor ?? payment.amountMinor} currency={payment.reportingCurrencyLabel} muted />
                        <p class="mt-1 text-[11px] uppercase tracking-[0.18em] text-slate-500">Reporting</p>
                      </div>
                      <button class="button-secondary" disabled={saving} on:click={() => deleteInvoicePayment(payment.id)} type="button">
                        Delete
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <div class="panel-soft p-4">
            <p class="label">Conversion snapshots</p>
            {#if selectedInvoiceDetail.conversions.length === 0}
              <p class="mt-3 text-sm text-slate-400">No conversion snapshots stored for this invoice yet.</p>
            {:else}
              <div class="mt-3 space-y-3">
                {#each selectedInvoiceDetail.conversions as conversion}
                  <div class="rounded-2xl bg-white/[0.03] px-4 py-3 text-sm">
                    <div class="flex flex-wrap items-center justify-between gap-3">
                      <p class="text-white">
                        {conversion.sourceCurrencyLabel} → {conversion.targetCurrencyLabel}
                      </p>
                      <MoneyValue amountMinor={conversion.convertedAmountMinor} currency={conversion.targetCurrencyLabel} />
                    </div>
                    <p class="mt-2 text-xs text-slate-400">
                      Rate {conversion.conversionRate} · source amount {formatMinorAmount(conversion.sourceAmountMinor)} · captured {conversion.capturedAt}
                    </p>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </SectionCard>

  {#if false}
  <SectionCard
    title="Invoice draft workflow"
    eyebrow="Draft workflow"
    description="Edit an existing draft or create a new one. Finalized invoices are shown read-only, can be duplicated before editing, and can be deleted from the invoice detail panel if you need to remove them."
  >
    <div class="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
      <form class="space-y-4" on:submit|preventDefault={saveInvoice}>
        <div class="grid gap-4 md:grid-cols-2">
          <label class="space-y-2">
            <span class="label">Business</span>
            <select class="input-base" bind:value={form.businessId} disabled={formLocked}>
              <option value="">Use active business</option>
              {#each businesses as business}
                <option value={String(business.id)}>
                  {business.businessName}{business.archivedAt ? ' (archived)' : ''}
                </option>
              {/each}
            </select>
          </label>
          <label class="space-y-2">
            <span class="label">Client</span>
            <select class="input-base" bind:value={form.clientId} disabled={formLocked}>
              <option value="">Select a client</option>
              {#each clients as client}
                <option value={String(client.id)}>
                  {client.companyName}{client.archivedAt ? ' (archived)' : ''}
                </option>
              {/each}
            </select>
          </label>
          <label class="space-y-2">
            <span class="label">Invoice number</span>
            <input class="input-base" bind:value={form.invoiceNumber} disabled={formLocked} placeholder="Leave blank for auto-numbering" />
          </label>
          <label class="space-y-2">
            <span class="label">Currency label</span>
            <input class="input-base" bind:value={form.currencyLabel} disabled={formLocked} placeholder="USD, Credits, Gold" />
          </label>
          <label class="space-y-2">
            <span class="label">Issue date</span>
            <input class="input-base" bind:value={form.issueDate} disabled={formLocked} type="date" />
          </label>
          <label class="space-y-2">
            <span class="label">Due date</span>
            <input class="input-base" bind:value={form.dueDate} disabled={formLocked} type="date" />
          </label>
          <label class="space-y-2 md:col-span-2">
            <span class="label">Notes</span>
            <textarea class="input-base min-h-[88px] resize-y" bind:value={form.notes} disabled={formLocked}></textarea>
          </label>
          <label class="space-y-2 md:col-span-2">
            <span class="label">Payment terms</span>
            <textarea class="input-base min-h-[88px] resize-y" bind:value={form.paymentTerms} disabled={formLocked}></textarea>
          </label>
        </div>

        <div class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <div>
              <p class="label">Line items</p>
              <p class="text-xs text-slate-500">Amounts are entered in major units and stored as integer minor units.</p>
            </div>
            <button class="button-secondary" disabled={formLocked} on:click={addLineItem} type="button">Add line</button>
          </div>

          <div class="space-y-3">
            {#each form.lineItems as lineItem, index}
              <div class="rounded-2xl border border-white/10 bg-white/[0.03] p-4">
                <div class="grid gap-3 lg:grid-cols-[1.4fr_0.7fr_0.7fr_auto]">
                  <label class="space-y-2">
                    <span class="label">Description</span>
                    <input class="input-base" bind:value={lineItem.description} disabled={formLocked} placeholder="Consulting services" />
                  </label>
                  <label class="space-y-2">
                    <span class="label">Quantity / hours</span>
                    <input class="input-base" bind:value={lineItem.quantity} disabled={formLocked} placeholder="1.5" />
                  </label>
                  <label class="space-y-2">
                    <span class="label">Rate</span>
                    <input class="input-base" bind:value={lineItem.rateMajor} disabled={formLocked} placeholder="150.00" />
                  </label>
                  <div class="flex items-end gap-2">
                    <button class="button-secondary" disabled={formLocked || index === 0} on:click={() => moveLineItem(index, -1)} type="button">
                      Up
                    </button>
                    <button
                      class="button-secondary"
                      disabled={formLocked || index === form.lineItems.length - 1}
                      on:click={() => moveLineItem(index, 1)}
                      type="button"
                    >
                      Down
                    </button>
                    <button class="button-secondary" disabled={formLocked && selectedInvoiceId !== null} on:click={() => removeLineItem(index)} type="button">
                      Remove
                    </button>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <button class="button-primary" disabled={saving || formLocked} type="submit">
            {selectedInvoiceId === null ? 'Create invoice' : 'Save draft'}
          </button>
          <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={duplicateSelectedInvoice} type="button">
            Duplicate
          </button>
          <button class="button-secondary" disabled={saving || !isDraftInvoice || selectedInvoiceId === null} on:click={finalizeSelectedInvoice} type="button">
            Finalize
          </button>
          <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={deleteSelectedInvoice} type="button">
            Delete invoice
          </button>
        </div>
      </form>

      <div class="space-y-4">
        <div class="panel-soft p-4">
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p class="label">Selected invoice</p>
                <h3 class="mt-1 text-lg font-semibold text-white">
                  {selectedInvoiceDetail?.invoice?.invoiceNumber ?? 'New invoice'}
                </h3>
              </div>
            {#if selectedInvoiceDetail}
              <StatusBadge status={selectedInvoiceDetail.invoice?.status ?? 'Draft'} />
            {/if}
          </div>

          {#if selectedInvoiceDetail}
            <div class="mt-4 grid gap-3 text-sm text-slate-300 md:grid-cols-2">
              <p><span class="text-slate-500">Business:</span> {selectedInvoiceDetail.invoice.businessSnapshot.businessName}</p>
              <p><span class="text-slate-500">Client:</span> {selectedInvoiceDetail.invoice.clientSnapshot.companyName}</p>
              <p><span class="text-slate-500">Issue:</span> {selectedInvoiceDetail.invoice.issueDate}</p>
              <p><span class="text-slate-500">Due:</span> {selectedInvoiceDetail.invoice.dueDate}</p>
              <p><span class="text-slate-500">Currency:</span> {selectedInvoiceDetail.invoice.currencyLabel}</p>
              <p><span class="text-slate-500">Status:</span> {selectedInvoiceDetail.invoice.status}</p>
            </div>
            <div class="mt-4 flex flex-wrap items-center gap-2">
              <label class="flex-1 space-y-2">
                <span class="label">PDF output directory</span>
                <input class="input-base" bind:value={pdfOutputDir} placeholder="Leave blank for the local exports folder" />
              </label>
            </div>
            <div class="mt-4 flex flex-wrap gap-2">
              <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={() => exportSelectedInvoicePdf(false)} type="button">
                Export PDF
              </button>
              <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={() => exportSelectedInvoicePdf(true)} type="button">
                Export &amp; open PDF
              </button>
              <button class="button-secondary" disabled={saving || selectedInvoiceId === null} on:click={exportSelectedInvoiceHtml} type="button">
                Export HTML
              </button>
            </div>
            {#if lastExportedPdfPath || lastExportedHtmlPath}
              <div class="mt-4 rounded-2xl border border-white/10 bg-white/[0.03] p-4">
                <p class="label">Last export</p>
                <div class="mt-3 space-y-2 text-sm text-slate-300">
                  {#if lastExportedPdfPath}
                    <p class="break-all"><span class="text-slate-500">PDF:</span> {lastExportedPdfPath}</p>
                  {/if}
                  {#if lastExportedHtmlPath}
                    <p class="break-all"><span class="text-slate-500">HTML:</span> {lastExportedHtmlPath}</p>
                  {/if}
                </div>
                <div class="mt-3 flex flex-wrap gap-2">
                  {#if lastExportedPdfPath}
                    <button class="button-secondary" disabled={saving} on:click={() => openExportedPath(lastExportedPdfPath)} type="button">
                      Open PDF
                    </button>
                  {/if}
                  {#if lastExportedHtmlPath}
                    <button class="button-secondary" disabled={saving} on:click={() => openExportedPath(lastExportedHtmlPath)} type="button">
                      Open HTML
                    </button>
                  {/if}
                  {#if lastExportedPdfPath || lastExportedHtmlPath}
                    <button
                      class="button-secondary"
                      disabled={saving}
                      on:click={() => openExportedPath(parentDirectory(lastExportedPdfPath ?? lastExportedHtmlPath ?? ''))}
                      type="button"
                    >
                      Open folder
                    </button>
                  {/if}
                </div>
              </div>
            {/if}
            {#if formLocked}
              <p class="mt-4 rounded-2xl border border-amber-400/20 bg-amber-400/10 px-3 py-2 text-xs text-amber-100">
                This invoice is finalized. Duplicate it to create a new editable draft.
              </p>
            {/if}
          {:else}
            <p class="mt-4 text-sm text-slate-400">
              Select a draft to edit it, or create a new invoice to start a local draft from the active business profile.
            </p>
          {/if}
        </div>

        <div class="space-y-4">
          {#if selectedInvoiceDetail}
            <div class="panel-soft p-4">
              <p class="label">Totals</p>
              <div class="mt-3 grid gap-3 sm:grid-cols-2">
                <div class="rounded-2xl border border-white/10 bg-white/[0.03] p-3">
                  <p class="text-xs text-slate-500">Subtotal</p>
                  <p class="mt-1 text-lg font-medium text-white">
                    <MoneyValue amountMinor={selectedInvoiceDetail.invoice.subtotalMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} />
                  </p>
                </div>
                <div class="rounded-2xl border border-white/10 bg-white/[0.03] p-3">
                  <p class="text-xs text-slate-500">Paid</p>
                  <p class="mt-1 text-lg font-medium text-white">
                    <MoneyValue amountMinor={selectedInvoiceDetail.invoice.paidMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} />
                  </p>
                </div>
                <div class="rounded-2xl border border-white/10 bg-white/[0.03] p-3">
                  <p class="text-xs text-slate-500">Outstanding</p>
                  <p class="mt-1 text-lg font-medium text-white">
                    <MoneyValue amountMinor={selectedInvoiceDetail.invoice.outstandingMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} />
                  </p>
                </div>
                <div class="rounded-2xl border border-white/10 bg-white/[0.03] p-3">
                  <p class="text-xs text-slate-500">Total</p>
                  <p class="mt-1 text-lg font-medium text-white">
                    <MoneyValue amountMinor={selectedInvoiceDetail.invoice.totalMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} />
                  </p>
                </div>
              </div>
            </div>

            <div class="panel-soft p-4">
              <p class="label">Line items</p>
              <div class="mt-3 overflow-hidden rounded-2xl border border-white/10">
                <table class="min-w-full divide-y divide-white/10 text-sm">
                  <thead class="bg-white/[0.03] text-left text-xs uppercase tracking-[0.16em] text-slate-400">
                    <tr>
                      <th class="px-4 py-3">Description</th>
                      <th class="px-4 py-3">Qty</th>
                      <th class="px-4 py-3">Rate</th>
                      <th class="px-4 py-3 text-right">Total</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-white/10">
                    {#each selectedInvoiceDetail.lineItems as lineItem}
                      <tr class="bg-white/[0.02]">
                        <td class="px-4 py-3 text-white">{lineItem.description}</td>
                        <td class="px-4 py-3 text-slate-300">{lineItem.quantity}</td>
                        <td class="px-4 py-3 text-slate-300"><MoneyValue amountMinor={lineItem.rateMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} /></td>
                        <td class="px-4 py-3 text-right text-slate-100"><MoneyValue amountMinor={lineItem.lineTotalMinor} currency={selectedInvoiceDetail.invoice.currencyLabel} /></td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>

            <div class="panel-soft p-4">
              <p class="label">Payments</p>
              {#if selectedInvoiceDetail.payments.length === 0}
                <p class="mt-3 text-sm text-slate-400">No payments recorded yet.</p>
              {:else}
                <div class="mt-3 space-y-3">
                  {#each selectedInvoiceDetail.payments as payment}
                    <div class="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-white/10 bg-white/[0.03] px-4 py-3">
                      <div>
                        <p class="font-medium text-white">{payment.paymentDate} · {payment.paymentSource}</p>
                        <p class="text-xs text-slate-400">
                          {payment.transactionReferenceId ?? 'No reference'}
                          {#if payment.conversionRate}
                            · rate {payment.conversionRate}
                          {/if}
                        </p>
                      </div>
                      <div class="flex flex-wrap items-center gap-3">
                        <div class="text-right">
                          <MoneyValue amountMinor={payment.amountMinor} currency={payment.currencyLabel} />
                          <p class="mt-1 text-[11px] uppercase tracking-[0.18em] text-slate-500">Source</p>
                          <MoneyValue amountMinor={payment.convertedAmountMinor ?? payment.amountMinor} currency={payment.reportingCurrencyLabel} muted />
                          <p class="mt-1 text-[11px] uppercase tracking-[0.18em] text-slate-500">Reporting</p>
                        </div>
                        <button class="button-secondary" disabled={saving} on:click={() => deleteInvoicePayment(payment.id)} type="button">
                          Delete
                        </button>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>

            <div class="panel-soft p-4">
              <p class="label">Conversion snapshots</p>
              {#if selectedInvoiceDetail.conversions.length === 0}
                <p class="mt-3 text-sm text-slate-400">No conversion snapshots stored for this invoice yet.</p>
              {:else}
                <div class="mt-3 space-y-3">
                  {#each selectedInvoiceDetail.conversions as conversion}
                    <div class="rounded-2xl border border-white/10 bg-white/[0.03] px-4 py-3 text-sm">
                      <div class="flex flex-wrap items-center justify-between gap-3">
                        <p class="text-white">
                          {conversion.sourceCurrencyLabel} → {conversion.targetCurrencyLabel}
                        </p>
                        <MoneyValue amountMinor={conversion.convertedAmountMinor} currency={conversion.targetCurrencyLabel} />
                      </div>
                      <p class="mt-2 text-xs text-slate-400">
                        Rate {conversion.conversionRate} · source amount {formatMinorAmount(conversion.sourceAmountMinor)} · captured {conversion.capturedAt}
                      </p>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </SectionCard>
  {/if}

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
