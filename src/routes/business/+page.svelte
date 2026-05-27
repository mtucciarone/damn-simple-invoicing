<script lang="ts">
  import { onMount } from 'svelte';

  import {
    archiveBusiness,
    createBusiness,
    getActiveBusiness,
    listBusinesses,
    setActiveBusiness,
    updateBusiness,
  } from '$lib/api/tauri';
  import SectionCard from '$lib/components/SectionCard.svelte';
  import type { BusinessProfile, BusinessProfileInput } from '$lib/types/domain';
  import { blankToNull } from '$lib/utils/money';

  type BusinessFormState = {
    businessName: string;
    legalName: string;
    address: string;
    country: string;
    email: string;
    phone: string;
    registrationNumber: string;
    taxVatNumber: string;
    logoPath: string;
    isActive: boolean;
  };

  const emptyBusinessForm = (business?: BusinessProfile | null): BusinessFormState => ({
    businessName: business?.businessName ?? '',
    legalName: business?.legalName ?? '',
    address: business?.address ?? '',
    country: business?.country ?? '',
    email: business?.email ?? '',
    phone: business?.phone ?? '',
    registrationNumber: business?.registrationNumber ?? '',
    taxVatNumber: business?.taxVatNumber ?? '',
    logoPath: business?.logoPath ?? '',
    isActive: business?.isActive ?? false,
  });

  let businesses: BusinessProfile[] = [];
  let activeBusiness: BusinessProfile | null = null;
  let includeArchived = false;
  let selectedBusinessId: number | null = null;
  let viewMode: 'list' | 'view' | 'edit' = 'list';
  let form: BusinessFormState = emptyBusinessForm();
  let loading = true;
  let saving = false;
  let error: string | null = null;
  let notice: string | null = null;

  $: selectedBusiness = businesses.find((business) => business.id === selectedBusinessId) ?? null;
  $: isReadOnly = viewMode === 'view';

  async function refreshBusinesses() {
    loading = true;
    error = null;
    try {
      [businesses, activeBusiness] = await Promise.all([
        listBusinesses(includeArchived),
        getActiveBusiness(),
      ]);
      if (selectedBusinessId !== null) {
        const refreshedSelected = businesses.find((business) => business.id === selectedBusinessId) ?? null;
        if (refreshedSelected) {
          form = emptyBusinessForm(refreshedSelected);
        }
      }
      if (!selectedBusinessId && !activeBusiness) {
        form.isActive = true;
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    await refreshBusinesses();
  });

  async function saveBusiness() {
    saving = true;
    error = null;
    notice = null;
    try {
      const input: BusinessProfileInput = {
        businessName: form.businessName.trim(),
        legalName: blankToNull(form.legalName),
        address: blankToNull(form.address),
        country: blankToNull(form.country),
        email: blankToNull(form.email),
        phone: blankToNull(form.phone),
        registrationNumber: blankToNull(form.registrationNumber),
        taxVatNumber: blankToNull(form.taxVatNumber),
        logoPath: blankToNull(form.logoPath),
        isActive: form.isActive,
      };

      const saved = selectedBusinessId === null
        ? await createBusiness(input)
        : await updateBusiness(selectedBusinessId, input);

      selectedBusinessId = saved.id;
      form = emptyBusinessForm(saved);
      notice = `${saved.businessName} saved locally.`;
      await refreshBusinesses();
      selectedBusinessId = saved.id;
      viewMode = 'view';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function activateBusiness(business: BusinessProfile) {
    saving = true;
    error = null;
    notice = null;
    try {
      const saved = await setActiveBusiness(business.id);
      notice = `${saved.businessName} is now the active business.`;
      await refreshBusinesses();
      selectedBusinessId = saved.id;
      form = emptyBusinessForm(saved);
      viewMode = 'view';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function archiveSelectedBusiness() {
    if (!selectedBusiness) {
      return;
    }

    if (!window.confirm(`Archive ${selectedBusiness.businessName}?`)) {
      return;
    }

    saving = true;
    error = null;
    notice = null;
    try {
      const saved = await archiveBusiness(selectedBusiness.id);
      notice = `${saved.businessName} archived locally.`;
      await refreshBusinesses();
      if (selectedBusinessId === saved.id) {
        form = emptyBusinessForm();
        selectedBusinessId = null;
        viewMode = 'list';
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  function startNewBusiness() {
    selectedBusinessId = null;
    form = emptyBusinessForm(activeBusiness ? { ...activeBusiness, isActive: false } : null);
    form.isActive = !activeBusiness;
    notice = null;
    error = null;
    viewMode = 'edit';
  }

  function backToBusinessList() {
    viewMode = 'list';
  }

  function viewSelectedBusiness(business: BusinessProfile) {
    selectedBusinessId = business.id;
    form = emptyBusinessForm(business);
    notice = null;
    error = null;
    viewMode = 'view';
  }

  function editSelectedBusiness() {
    if (selectedBusinessId !== null) {
      viewMode = 'edit';
    }
  }

  function backToBusinessView() {
    if (selectedBusinessId !== null) {
      const refreshedSelected = businesses.find((business) => business.id === selectedBusinessId) ?? null;
      if (refreshedSelected) {
        form = emptyBusinessForm(refreshedSelected);
      }
      viewMode = 'view';
    } else {
      viewMode = 'list';
    }
  }
</script>

<div class="space-y-6">
  <SectionCard
    title="Business profile"
    description="Create one or more local business profiles. The active profile is snapped into new invoices."
  >
    <svelte:fragment slot="actions">
      {#if viewMode === 'view'}
        <button class="button-secondary" on:click={editSelectedBusiness} type="button">Edit</button>
        <button class="button-secondary" on:click={backToBusinessList} type="button">Back to list</button>
      {:else if viewMode === 'edit'}
        {#if selectedBusinessId !== null}
          <button class="button-secondary" on:click={backToBusinessView} type="button">View</button>
        {/if}
        <button class="button-secondary" on:click={backToBusinessList} type="button">Back to list</button>
      {:else}
        <button class="button-secondary" on:click={startNewBusiness} type="button">New profile</button>
      {/if}
      <button class="button-secondary" disabled={saving} on:click={refreshBusinesses} type="button">Refresh</button>
    </svelte:fragment>

    {#if viewMode === 'list'}
      <div class="space-y-4">
        <div class="panel-soft p-4">
          <p class="label">Active profile</p>
          {#if activeBusiness}
            <div class="mt-3 flex flex-wrap items-center justify-between gap-3">
              <div>
                <p class="text-base font-medium text-white">{activeBusiness.businessName}</p>
                <p class="text-sm text-slate-400">{activeBusiness.legalName ?? 'No legal name set'}</p>
                <p class="text-sm text-slate-400">{activeBusiness.country ?? 'No country set'}</p>
              </div>
              <span class="rounded-full border border-emerald-400/20 bg-emerald-400/10 px-2.5 py-1 text-[11px] font-medium text-emerald-200">
                Active
              </span>
            </div>
          {:else}
            <p class="mt-3 text-sm text-slate-400">No active business yet.</p>
          {/if}
        </div>

        <div class="panel-soft p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="label">Profiles</p>
              <p class="text-sm text-slate-400">Select a profile to view, edit, or activate it.</p>
            </div>
            <label class="flex items-center gap-2 text-xs text-slate-400">
              <input bind:checked={includeArchived} on:change={refreshBusinesses} type="checkbox" />
              Show archived
            </label>
          </div>

          {#if loading}
            <div class="py-6 text-sm text-slate-400">Loading businesses from local SQLite...</div>
          {:else if businesses.length === 0}
            <div class="py-6 text-sm text-slate-400">No business profiles have been created yet.</div>
          {:else}
            <div class="mt-4 divide-y divide-white/10">
              {#each businesses as business}
                <div
                  class={`flex flex-wrap items-center justify-between gap-3 py-3 transition ${selectedBusinessId === business.id ? 'bg-accent-500/10 px-3 rounded-xl' : ''}`}
                >
                  <div class="min-w-0">
                    <p class="font-medium text-white">{business.businessName}</p>
                    <p class="text-xs text-slate-400">
                      {business.legalName ?? 'No legal name'}
                      {#if business.country}
                        · {business.country}
                      {/if}
                    </p>
                  </div>
                  <div class="flex items-center gap-2">
                    {#if business.isActive}
                      <span class="rounded-full border border-emerald-400/20 bg-emerald-400/10 px-2 py-1 text-[11px] font-medium text-emerald-200">
                        Active
                      </span>
                    {/if}
                    {#if business.archivedAt}
                      <span class="rounded-full border border-slate-700 bg-slate-800 px-2 py-1 text-[11px] font-medium text-slate-300">
                        Archived
                      </span>
                    {/if}
                    <button class="button-secondary" disabled={saving} on:click={() => viewSelectedBusiness(business)} type="button">
                      View
                    </button>
                    <button class="button-secondary" disabled={saving} on:click={() => activateBusiness(business)} type="button">
                      Activate
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <div class="space-y-6">
        <div class="max-w-4xl space-y-4">
          <div class="panel-soft p-4">
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p class="label">{isReadOnly ? 'Viewing profile' : 'Editing profile'}</p>
                <h3 class="mt-1 text-lg font-semibold text-white">{selectedBusiness?.businessName ?? 'New profile'}</h3>
              </div>
              <div class="flex flex-wrap gap-2">
                {#if selectedBusiness?.isActive}
                  <span class="rounded-full border border-emerald-400/20 bg-emerald-400/10 px-2.5 py-1 text-[11px] font-medium text-emerald-200">
                    Active
                  </span>
                {/if}
                {#if selectedBusiness?.archivedAt}
                  <span class="rounded-full border border-slate-700 bg-slate-800 px-2.5 py-1 text-[11px] font-medium text-slate-300">
                    Archived
                  </span>
                {/if}
                {#if isReadOnly && selectedBusinessId !== null}
                  <button class="button-primary" disabled={saving} on:click={editSelectedBusiness} type="button">Edit</button>
                {/if}
              </div>
            </div>

            {#if selectedBusiness}
              <div class="mt-4 grid gap-3 text-sm text-slate-300 md:grid-cols-2">
                <p><span class="text-slate-500">Legal name:</span> {selectedBusiness.legalName ?? '—'}</p>
                <p><span class="text-slate-500">Country:</span> {selectedBusiness.country ?? '—'}</p>
                <p><span class="text-slate-500">Email:</span> {selectedBusiness.email ?? '—'}</p>
                <p><span class="text-slate-500">Phone:</span> {selectedBusiness.phone ?? '—'}</p>
                <p><span class="text-slate-500">Registration:</span> {selectedBusiness.registrationNumber ?? '—'}</p>
                <p><span class="text-slate-500">Tax / VAT:</span> {selectedBusiness.taxVatNumber ?? '—'}</p>
              </div>
            {/if}
          </div>

          <form class="space-y-4 max-w-4xl panel-soft p-4" on:submit|preventDefault={saveBusiness}>
            <div class="grid gap-4 md:grid-cols-2">
              <label class="space-y-2">
                <span class="label">Business name</span>
                <input class="input-base" bind:value={form.businessName} placeholder="Damn Simple Studio" disabled={isReadOnly} />
              </label>
              <label class="space-y-2">
                <span class="label">Legal name</span>
                <input class="input-base" bind:value={form.legalName} placeholder="Damn Simple Studio LLC" disabled={isReadOnly} />
              </label>
              <label class="space-y-2 md:col-span-2">
                <span class="label">Address</span>
                <textarea class="input-base min-h-[92px] resize-y" bind:value={form.address} placeholder="Street, city, postcode" disabled={isReadOnly}></textarea>
              </label>
              <label class="space-y-2">
                <span class="label">Country</span>
                <input class="input-base" bind:value={form.country} placeholder="Indonesia" disabled={isReadOnly} />
              </label>
              <label class="space-y-2">
                <span class="label">Email</span>
                <input class="input-base" bind:value={form.email} type="email" placeholder="billing@example.com" disabled={isReadOnly} />
              </label>
              <label class="space-y-2">
                <span class="label">Phone</span>
                <input class="input-base" bind:value={form.phone} placeholder="+62 000 0000" disabled={isReadOnly} />
              </label>
              <label class="space-y-2">
                <span class="label">Registration number</span>
                <input class="input-base" bind:value={form.registrationNumber} placeholder="REG-001" disabled={isReadOnly} />
              </label>
              <label class="space-y-2">
                <span class="label">Tax / VAT number</span>
                <input class="input-base" bind:value={form.taxVatNumber} placeholder="VAT-12345" disabled={isReadOnly} />
              </label>
              <label class="space-y-2 md:col-span-2">
                <span class="label">Logo path</span>
                <input class="input-base" bind:value={form.logoPath} placeholder="C:\\Users\\me\\logo.png" disabled={isReadOnly} />
              </label>
            </div>

            <label class="flex items-center gap-3 rounded-2xl bg-white/[0.03] px-4 py-3 text-sm text-slate-200">
              <input bind:checked={form.isActive} class="h-4 w-4" type="checkbox" disabled={isReadOnly} />
              Set as active business profile
            </label>

            <div class="flex flex-wrap items-center gap-3">
              {#if isReadOnly}
                <button class="button-primary" disabled={saving || selectedBusinessId === null} on:click={editSelectedBusiness} type="button">
                  Edit
                </button>
              {:else}
                <button class="button-primary" disabled={saving} type="submit">
                  {selectedBusinessId === null ? 'Create profile' : 'Save changes'}
                </button>
              {/if}
              <button class="button-secondary" disabled={saving || selectedBusinessId === null} on:click={archiveSelectedBusiness} type="button">
                Archive selected
              </button>
              <p class="text-xs text-slate-500">
                New invoices snapshot the active business at creation time.
              </p>
            </div>
          </form>

          <div class="grid gap-4 lg:grid-cols-2">
            <div class="space-y-4">
              <div class="panel-soft p-4">
                <p class="label">Selected profile</p>
                {#if selectedBusiness}
                  <div class="mt-3 space-y-2">
                    <p class="text-base font-medium text-white">{selectedBusiness.businessName}</p>
                    <p class="text-sm text-slate-400">{selectedBusiness.legalName ?? 'No legal name set'}</p>
                    <p class="text-sm text-slate-400">{selectedBusiness.country ?? 'No country set'}</p>
                    <div class="flex flex-wrap gap-2 pt-2">
                      {#if selectedBusiness.isActive}
                        <span class="rounded-full border border-emerald-400/20 bg-emerald-400/10 px-2.5 py-1 text-[11px] font-medium text-emerald-200">
                          Active
                        </span>
                      {/if}
                      {#if selectedBusiness.archivedAt}
                        <span class="rounded-full border border-slate-700 bg-slate-800 px-2.5 py-1 text-[11px] font-medium text-slate-300">
                          Archived
                        </span>
                      {/if}
                    </div>
                  </div>
                {:else}
                  <p class="mt-3 text-sm text-slate-400">No profile selected.</p>
                {/if}
              </div>

              <button class="button-secondary w-full justify-center" on:click={backToBusinessList} type="button">
                Back to list
              </button>
            </div>

            <div class="panel-soft p-4 text-sm text-slate-400">
              New invoices snapshot the active profile at creation time. Switch the active profile before creating a new invoice if the business identity changes.
            </div>
          </div>
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
