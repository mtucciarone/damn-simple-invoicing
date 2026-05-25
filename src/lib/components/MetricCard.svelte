<script lang="ts">
  export let label: string;
  export let value: string;
  export let detail: string | null = null;
  export let tone: 'default' | 'accent' | 'warm' = 'default';

  const toneClass = {
    default: 'from-white/5 to-white/[0.02]',
    accent: 'from-accent-500/20 to-white/[0.03]',
    warm: 'from-warm-500/20 to-white/[0.03]',
  } as const;
</script>

<article class={`panel-soft relative overflow-hidden bg-gradient-to-br p-4 ${toneClass[tone]}`}>
  <div class="pointer-events-none absolute right-0 top-0 h-24 w-24 translate-x-1/3 -translate-y-1/3 rounded-full bg-white/10 blur-2xl"></div>
  <div class="relative">
    <p class="label">{label}</p>
    <p class="mt-3 text-2xl font-semibold tracking-tight text-white tabular-nums">{value}</p>
    {#if detail}
      <p class="mt-2 text-sm text-slate-400">{detail}</p>
    {/if}
  </div>
</article>
