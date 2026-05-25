<script lang="ts">
  import type { InvoiceStatus } from '$lib/types/domain';

  export let status: InvoiceStatus;

  const classes: Record<InvoiceStatus, string> = {
    Draft: 'border-white/10 bg-white/5 text-slate-300',
    Sent: 'border-accent-400/30 bg-accent-400/10 text-accent-200',
    'Partially Paid': 'border-warm-400/30 bg-warm-400/10 text-warm-200',
    Paid: 'border-emerald-400/30 bg-emerald-400/10 text-emerald-200',
    Overdue: 'border-rose-400/30 bg-rose-400/10 text-rose-200',
    Cancelled: 'border-slate-600 bg-slate-800 text-slate-300',
  };
</script>

<span class={`inline-flex items-center whitespace-nowrap rounded-full border px-2.5 py-1 text-xs font-semibold tracking-wide tabular-nums ${classes[status]}`}>
  {status}
</span>
