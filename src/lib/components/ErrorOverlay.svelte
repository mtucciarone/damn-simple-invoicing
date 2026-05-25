<script lang="ts">
  import type { CapturedError } from '$lib/utils/errors';

  export let error: CapturedError;
  export let primaryLabel = 'Retry';
  export let secondaryLabel: string | null = 'Dismiss';
  export let showSecondary = true;
  export let onPrimary: (() => void) | null = null;
  export let onSecondary: (() => void) | null = null;
</script>

<div class="fixed inset-0 z-[200] flex items-center justify-center px-4 py-6">
  <div class="absolute inset-0 bg-ink-950/80 backdrop-blur-md"></div>

  <section
    aria-describedby="app-error-description"
    aria-labelledby="app-error-title"
    aria-modal="true"
    class="relative z-10 w-full max-w-3xl overflow-hidden rounded-[2rem] border border-white/10 bg-ink-950/95 shadow-[0_30px_100px_rgba(2,6,23,0.7)]"
    role="alertdialog"
  >
    <div class="border-b border-white/10 bg-gradient-to-r from-rose-500/15 via-transparent to-accent-400/10 px-6 py-5">
      <p class="label">Application alert</p>
      <h2 id="app-error-title" class="mt-2 text-2xl font-semibold text-white">
        {error.title}
      </h2>
      <p id="app-error-description" class="mt-2 whitespace-pre-wrap break-words text-sm leading-6 text-slate-300">
        {error.message}
      </p>
      <p class="mt-2 text-xs text-slate-500">
        {error.name}
        {#if error.cause}
          · {error.cause}
        {/if}
      </p>
    </div>

    <div class="space-y-4 px-6 py-5">
      {#if error.stack}
        <details class="rounded-2xl border border-white/10 bg-white/[0.03] p-4">
          <summary class="cursor-pointer text-sm font-medium text-white">Technical details</summary>
          <pre class="mt-3 max-h-[42vh] overflow-auto whitespace-pre-wrap break-words text-xs leading-6 text-slate-300">{error.stack}</pre>
        </details>
      {/if}

      <div class="flex flex-wrap items-center gap-3">
        <button autofocus class="button-primary" on:click={() => onPrimary?.()} type="button">
          {primaryLabel}
        </button>

        {#if showSecondary && secondaryLabel}
          <button class="button-secondary" on:click={() => onSecondary?.()} type="button">
            {secondaryLabel}
          </button>
        {/if}
      </div>
    </div>
  </section>
</div>
