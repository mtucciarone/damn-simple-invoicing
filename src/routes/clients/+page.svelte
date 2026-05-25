<script lang="ts">
  import { onMount } from 'svelte';

  import {
    archiveClient,
    createClient,
    deleteClient,
    getClient,
    listClients,
    updateClient,
  } from '$lib/api/tauri';
  import MoneyValue from '$lib/components/MoneyValue.svelte';
  import SectionCard from '$lib/components/SectionCard.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import type { Client, ClientDetail, ClientInput } from '$lib/types/domain';
  import { blankToNull } from '$lib/utils/money';

  type ClientFormState = {
    companyName: string;
    contactPerson: string;
    email: string;
    address: string;
    country: string;
    notes: string;
  };

  const emptyClientForm = (client?: Client | null): ClientFormState => ({
    companyName: client?.companyName ?? '',
    contactPerson: client?.contactPerson ?? '',
    email: client?.email ?? '',
    address: client?.address ?? '',
    country: client?.country ?? '',
    notes: client?.notes ?? '',
  });

  let clients: Client[] = [];
  let selectedClientId: number | null = null;
  let clientDetail: ClientDetail | null = null;
  let viewMode: 'list' | 'view' | 'edit' = 'list';
  let search = '';
  let includeArchived = false;
  let form: ClientFormState = emptyClientForm();
  let loading = true;
  let saving = false;
  let error: string | null = null;
  let notice: string | null = null;

  $: selectedClient = clientDetail?.client ?? clients.find((client) => client.id === selectedClientId) ?? null;
  $: isReadOnly = viewMode === 'view';

  async function refreshClients() {
    loading = true;
    error = null;
    try {
      clients = await listClients(search.trim() || null, includeArchived);
      if (selectedClientId !== null) {
        clientDetail = await getClient(selectedClientId);
        form = emptyClientForm(clientDetail?.client ?? null);
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  async function loadClientDetail(clientId: number | null, nextMode: 'view' | 'edit' = 'view') {
    selectedClientId = clientId;
    notice = null;
    error = null;
    viewMode = nextMode;
    if (clientId === null) {
      clientDetail = null;
      form = emptyClientForm();
      return;
    }

    try {
      clientDetail = await getClient(clientId);
      form = emptyClientForm(clientDetail?.client ?? null);
    } catch (cause) {
      clientDetail = null;
      selectedClientId = null;
      form = emptyClientForm();
      viewMode = 'list';
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function startNewClient() {
    selectedClientId = null;
    clientDetail = null;
    form = emptyClientForm();
    notice = null;
    error = null;
    viewMode = 'edit';
  }

  function backToClientList() {
    viewMode = 'list';
  }

  async function viewSelectedClient() {
    if (selectedClientId !== null) {
      await loadClientDetail(selectedClientId, 'view');
    }
  }

  async function editSelectedClient() {
    if (selectedClientId !== null) {
      await loadClientDetail(selectedClientId, 'edit');
    }
  }

  onMount(async () => {
    await refreshClients();
  });

  async function saveClient() {
    if (isReadOnly) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      const input: ClientInput = {
        companyName: form.companyName.trim(),
        contactPerson: blankToNull(form.contactPerson),
        email: blankToNull(form.email),
        address: blankToNull(form.address),
        country: blankToNull(form.country),
        notes: blankToNull(form.notes),
      };

      const saved = selectedClientId === null
        ? await createClient(input)
        : await updateClient(selectedClientId, input);

      selectedClientId = saved.id;
      clientDetail = await getClient(saved.id);
      form = emptyClientForm(clientDetail?.client ?? null);
      notice = `${saved.companyName} saved locally.`;
      await refreshClients();
      selectedClientId = saved.id;
      clientDetail = await getClient(saved.id);
      form = emptyClientForm(clientDetail?.client ?? null);
      viewMode = 'view';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function archiveSelectedClient() {
    if (!selectedClient) {
      return;
    }

    if (!window.confirm(`Archive ${selectedClient.companyName}?`)) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      const saved = await archiveClient(selectedClient.id);
      notice = `${saved.companyName} archived locally.`;
      selectedClientId = saved.id;
      clientDetail = await getClient(saved.id);
      form = emptyClientForm(clientDetail?.client ?? null);
      await refreshClients();
      viewMode = 'view';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function deleteSelectedClient() {
    if (!selectedClient) {
      return;
    }

    if (!window.confirm(`Delete ${selectedClient.companyName}? This only works when the client has no invoice history.`)) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      await deleteClient(selectedClient.id);
      notice = `${selectedClient.companyName} deleted locally.`;
      selectedClientId = null;
      clientDetail = null;
      form = emptyClientForm();
      await refreshClients();
      viewMode = 'list';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }
</script>

<div class="space-y-6">
  <SectionCard
    title="Clients"
    eyebrow="CRUD"
    description="Search, view, edit, archive, and review invoice history for each local client record."
  >
    <svelte:fragment slot="actions">
      {#if viewMode === 'view'}
        <button class="button-secondary" on:click={editSelectedClient} type="button">Edit</button>
        <button class="button-secondary" on:click={backToClientList} type="button">Back to list</button>
      {:else if viewMode === 'edit'}
        {#if selectedClientId !== null}
          <button class="button-secondary" on:click={viewSelectedClient} type="button">View</button>
        {/if}
        <button class="button-secondary" on:click={backToClientList} type="button">Back to list</button>
      {:else}
        <button class="button-secondary" on:click={startNewClient} type="button">New client</button>
      {/if}
      <button class="button-secondary" disabled={saving} on:click={refreshClients} type="button">Refresh</button>
    </svelte:fragment>

    {#if viewMode === 'list'}
      <div class="space-y-4">
        <div class="panel-soft p-4">
          <div class="flex flex-wrap items-end gap-3">
            <label class="flex-1 space-y-2">
              <span class="label">Search</span>
              <input
                class="input-base"
                bind:value={search}
                placeholder="Search company, contact, email"
                on:keydown={(event) => event.key === 'Enter' && refreshClients()}
              />
            </label>
            <button class="button-primary" disabled={loading} on:click={refreshClients} type="button">
              Apply filters
            </button>
            <label class="flex items-center gap-2 text-xs text-slate-400">
              <input bind:checked={includeArchived} on:change={refreshClients} type="checkbox" />
              Show archived
            </label>
          </div>
        </div>

        <div class="panel-soft p-4">
          <div class="flex items-center justify-between gap-3">
            <div>
              <p class="label">Client list</p>
              <p class="text-sm text-slate-400">Select a client to open the read-only view or edit it.</p>
            </div>
            <p class="text-xs text-slate-500">{clients.length} records</p>
          </div>

          {#if loading}
            <div class="py-6 text-sm text-slate-400">Loading clients from local SQLite...</div>
          {:else if clients.length === 0}
            <div class="py-6 text-sm text-slate-400">No clients found.</div>
          {:else}
            <div class="mt-4 overflow-hidden rounded-2xl bg-white/[0.02]">
              <table class="min-w-full divide-y divide-white/10 text-sm">
                <thead class="bg-white/[0.03] text-left text-xs uppercase tracking-[0.16em] text-slate-400">
                  <tr>
                    <th class="px-4 py-3">Company</th>
                    <th class="px-4 py-3">Contact</th>
                    <th class="px-4 py-3">Email</th>
                    <th class="px-4 py-3 text-right">Status</th>
                    <th class="px-4 py-3">Actions</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-white/10">
                  {#each clients as client}
                    <tr class={`transition hover:bg-white/[0.04] ${selectedClientId === client.id ? 'bg-accent-500/10' : 'bg-white/[0.02]'}`}>
                      <td class="px-4 py-3">
                        <p class="font-medium text-white">{client.companyName}</p>
                      </td>
                      <td class="px-4 py-3 text-slate-300">{client.contactPerson ?? 'No contact'}</td>
                      <td class="px-4 py-3 text-slate-300">{client.email ?? '—'}</td>
                      <td class="px-4 py-3 text-right">
                        {#if client.archivedAt}
                          <span class="rounded-full border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] font-medium text-slate-300">
                            Archived
                          </span>
                        {:else}
                          <span class="text-xs text-slate-500">Active</span>
                        {/if}
                      </td>
                      <td class="px-4 py-3">
                        <button class="button-secondary" disabled={saving} on:click={() => loadClientDetail(client.id, 'view')} type="button">
                          View
                        </button>
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
              <p class="label">{isReadOnly ? 'Viewing client' : 'Editing client'}</p>
              <h3 class="mt-1 text-lg font-semibold text-white">{selectedClient?.companyName ?? 'New client'}</h3>
            </div>
            {#if selectedClient?.archivedAt}
              <span class="rounded-full border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] font-medium text-slate-300">
                Archived
              </span>
            {/if}
          </div>

          <div class="mt-4 grid gap-3 text-sm text-slate-300 md:grid-cols-2">
            <p><span class="text-slate-500">Contact:</span> {selectedClient?.contactPerson ?? '—'}</p>
            <p><span class="text-slate-500">Email:</span> {selectedClient?.email ?? '—'}</p>
            <p><span class="text-slate-500">Country:</span> {selectedClient?.country ?? '—'}</p>
            <p><span class="text-slate-500">Notes:</span> {selectedClient?.notes ?? '—'}</p>
          </div>
        </div>

        <form class="panel-soft space-y-4 p-4" on:submit|preventDefault={saveClient}>
          <div class="grid gap-4 md:grid-cols-2">
            <label class="space-y-2 md:col-span-2">
              <span class="label">Company name</span>
              <input class="input-base" bind:value={form.companyName} disabled={isReadOnly} placeholder="Acme Studio" />
            </label>
            <label class="space-y-2">
              <span class="label">Contact person</span>
              <input class="input-base" bind:value={form.contactPerson} disabled={isReadOnly} placeholder="A. Person" />
            </label>
            <label class="space-y-2">
              <span class="label">Email</span>
              <input class="input-base" bind:value={form.email} disabled={isReadOnly} type="email" placeholder="client@example.com" />
            </label>
            <label class="space-y-2 md:col-span-2">
              <span class="label">Address</span>
              <textarea class="input-base min-h-[92px] resize-y" bind:value={form.address} disabled={isReadOnly}></textarea>
            </label>
            <label class="space-y-2">
              <span class="label">Country</span>
              <input class="input-base" bind:value={form.country} disabled={isReadOnly} placeholder="Indonesia" />
            </label>
            <label class="space-y-2 md:col-span-2">
              <span class="label">Notes</span>
              <textarea class="input-base min-h-[110px] resize-y" bind:value={form.notes} disabled={isReadOnly} placeholder="Optional internal notes"></textarea>
            </label>
          </div>

          <div class="flex flex-wrap items-center gap-3">
            {#if isReadOnly}
              <button class="button-primary" disabled={saving || selectedClientId === null} on:click={editSelectedClient} type="button">
                Edit
              </button>
            {:else}
              <button class="button-primary" disabled={saving} type="submit">
                {selectedClientId === null ? 'Create client' : 'Save changes'}
              </button>
              {#if selectedClientId !== null}
                <button class="button-secondary" disabled={saving} on:click={viewSelectedClient} type="button">
                  View
                </button>
              {/if}
            {/if}
            <button class="button-secondary" disabled={saving || selectedClientId === null} on:click={archiveSelectedClient} type="button">
              Archive selected
            </button>
            <button class="button-secondary" disabled={saving || selectedClientId === null} on:click={deleteSelectedClient} type="button">
              Delete selected
            </button>
          </div>
        </form>

        <div class="panel-soft p-4">
          <p class="label">Invoice history</p>
          <p class="mt-1 text-sm text-slate-400">Invoices remain immutable after finalization, so this history is safe to inspect later.</p>
          {#if clientDetail && clientDetail.invoiceHistory.length > 0}
            <div class="mt-4 overflow-hidden rounded-2xl bg-white/[0.02]">
              <table class="min-w-full divide-y divide-white/10 text-sm">
                <thead class="bg-white/[0.03] text-left text-xs uppercase tracking-[0.16em] text-slate-400">
                  <tr>
                    <th class="px-4 py-3">Invoice</th>
                    <th class="px-4 py-3">Status</th>
                    <th class="px-4 py-3">Issue</th>
                    <th class="px-4 py-3">Due</th>
                    <th class="px-4 py-3 text-right">Total</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-white/10">
                  {#each clientDetail.invoiceHistory as invoice}
                    <tr class="bg-white/[0.02]">
                      <td class="px-4 py-3 font-medium text-white">{invoice.invoiceNumber}</td>
                      <td class="px-4 py-3"><StatusBadge status={invoice.status} /></td>
                      <td class="px-4 py-3 text-slate-300">{invoice.issueDate}</td>
                      <td class="px-4 py-3 text-slate-300">{invoice.dueDate}</td>
                      <td class="px-4 py-3 text-right"><MoneyValue amountMinor={invoice.totalMinor} currency={invoice.currencyLabel} /></td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {:else}
            <p class="mt-4 text-sm text-slate-400">No invoice history for the selected client.</p>
          {/if}
        </div>
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
