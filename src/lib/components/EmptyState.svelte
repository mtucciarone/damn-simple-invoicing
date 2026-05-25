<script lang="ts">
  export let title: string;
  export let description: string;
  export let actionLabel: string | null = null;
  export let href: string | null = null;
</script>

<div class="panel relative overflow-hidden p-6">
  <div class="pointer-events-none absolute right-0 top-0 h-32 w-32 translate-x-1/3 -translate-y-1/3 rounded-full bg-accent-400/10 blur-3xl"></div>
  <div class="relative flex flex-col items-start gap-4">
    <div class="max-w-2xl">
      <p class="label">Empty state</p>
      <h3 class="mt-2 text-xl font-semibold text-white">{title}</h3>
      <p class="mt-2 text-sm text-slate-400">{description}</p>
    </div>

    {#if actionLabel && href}
      <a class="button-primary" href={href}>{actionLabel}</a>
    {/if}

    <slot />
  </div>
</div>
