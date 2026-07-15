<script lang="ts">
  import { moneyFormatPreference } from '$lib/stores/settings';
  import { formatMinorAmount } from '$lib/utils/money';

  export let amountMinor: number;
  export let currency: string;
  export let muted = false;
</script>

<span class={muted ? 'text-slate-400 tabular-nums' : 'text-white tabular-nums'}>
  {#if currency}
    {formatMinorAmount(amountMinor, $moneyFormatPreference)} {currency}
  {:else}
    {formatMinorAmount(amountMinor, $moneyFormatPreference)}
  {/if}
</span>
