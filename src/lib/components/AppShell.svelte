<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";

  import { getAppState } from "$lib/api/tauri";
  import type { AppBootState } from "$lib/types/domain";

  type NavItem = {
    href: string;
    label: string;
    hint: string;
  };

  const nav: NavItem[] = [
    { href: "/", label: "Dashboard", hint: "At a glance" },
    { href: "/business", label: "Business", hint: "Profile & branding" },
    { href: "/clients", label: "Clients", hint: "Contacts & history" },
    { href: "/invoices", label: "Invoices", hint: "Drafts & locks" },
    { href: "/payments", label: "Payments", hint: "Collections" },
    { href: "/reports", label: "Reports", hint: "Numbers & filters" },
    { href: "/backups", label: "Backups", hint: "Export & restore" },
    { href: "/settings", label: "Settings", hint: "Defaults & storage" },
  ];

  let appState: AppBootState | null = null;
  let appError: string | null = null;

  onMount(async () => {
    try {
      appState = await getAppState();
    } catch (error) {
      appError = error instanceof Error ? error.message : String(error);
    }
  });

  const isActive = (href: string) =>
    href === "/"
      ? page.url.pathname === "/"
      : page.url.pathname === href || page.url.pathname.startsWith(`${href}/`);

  const currentItem = () => nav.find((item) => isActive(item.href)) ?? nav[0];

  const currentTitle = () =>
    page.url.pathname === "/"
      ? "Dashboard"
      : (currentItem()?.label ?? "Workspace");

  const currentSubtitle = () =>
    page.url.pathname === "/"
      ? "Unified view of invoices, payments, local reporting, and export actions."
      : (currentItem()?.hint ?? "Offline workspace");
</script>

<div class="relative min-h-screen overflow-hidden bg-mesh-dark text-slate-100">
  <div
    class="pointer-events-none absolute inset-0"
    style="background: radial-gradient(circle at top left, rgba(34, 211, 238, 0.08), transparent 28%), radial-gradient(circle at top right, rgba(249, 115, 22, 0.06), transparent 26%);"
  ></div>
  <div
    class="pointer-events-none absolute inset-0"
    style="background: linear-gradient(180deg, rgba(5, 8, 22, 0.2), rgba(5, 8, 22, 0.85));"
  ></div>

  <div
    class="relative mx-auto grid min-h-screen max-w-[1920px] lg:grid-cols-[330px_1fr]"
  >
    <aside
      class="sticky top-0 flex h-screen flex-col border-b border-white/10 bg-black/30 p-5 backdrop-blur-2xl lg:border-b-0 lg:border-r"
    >
      <div
        class="rounded-[1.75rem] border border-white/10 bg-white/[0.05] p-5 shadow-panel"
      >
        <div class="flex items-center gap-3">
          <div
            class="flex h-11 w-11 items-center justify-center rounded-2xl bg-gradient-to-br from-accent-400 via-accent-500 to-warm-400 text-sm font-semibold text-ink-950 shadow-lg shadow-accent-500/20"
          >
            DS
          </div>
          <div>
            <h1 class="mt-1 text-xl font-semibold tracking-tight text-white">
              Damn Simple Invoicing
            </h1>
          </div>
        </div>
        <div class="p-4">
          <p class="label">Active Business</p>
          {#if appState}
            <div class="mt-3 space-y-3">
              <div>
                <p class="text-sm font-medium text-white">
                  {appState.activeBusiness?.businessName ??
                    "No active business profile"}
                </p>
                <p class="mt-1 text-xs text-slate-500">
                  Reporting currency: {appState.reportingCurrencyLabel}
                </p>
              </div>
            </div>
          {:else if appError}
            <p class="mt-3 text-sm text-rose-200">{appError}</p>
          {:else}
            <p class="mt-3 text-sm text-slate-400">Loading local state...</p>
          {/if}
        </div>
      </div>

      <nav class="mt-6 flex-1 space-y-2 overflow-y-auto pr-1">
        {#each nav as item}
          <a
            href={item.href}
            class={`group relative block overflow-hidden rounded-2xl border px-4 py-3 transition ${
              isActive(item.href)
                ? "border-accent-400/30 bg-accent-500/15 text-white shadow-soft"
                : "border-white/10 bg-white/[0.03] text-slate-300 hover:border-white/20 hover:bg-white/[0.06]"
            }`}
            aria-current={isActive(item.href) ? "page" : undefined}
          >
            <div
              class={`absolute inset-y-0 left-0 w-1 rounded-r-full bg-accent-400 transition ${
                isActive(item.href)
                  ? "opacity-100"
                  : "opacity-0 group-hover:opacity-60"
              }`}
            ></div>
            <div class="flex items-start justify-between gap-3 pl-1">
              <div>
                <span class="block text-sm font-medium">{item.label}</span>
                <span class="mt-1 block text-xs text-slate-500"
                  >{item.hint}</span
                >
              </div>
            </div>
          </a>
        {/each}
      </nav>
    </aside>

    <main class="relative p-5 lg:p-8">
      <slot />
    </main>
  </div>
</div>
