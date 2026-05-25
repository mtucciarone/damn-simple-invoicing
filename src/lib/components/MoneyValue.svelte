<script lang="ts">
  export let amountMinor: number;
  export let currency: string;
  export let muted = false;

  const formatAmount = (value: number) => (value / 100).toFixed(2);
</script>

<span class={muted ? 'text-slate-400 tabular-nums' : 'text-white tabular-nums'}>
  {#if currency}
    {formatAmount(amountMinor)} {currency}
  {:else}
    {formatAmount(amountMinor)}
  {/if}
</span>
