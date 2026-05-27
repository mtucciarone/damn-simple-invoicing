<script lang="ts">
  export let title: string;
  export let description: string | null = null;
</script>

<section class="panel">
  <div class="flex items-start justify-between gap-4 border-b border-white/5 bg-white/[0.025] px-5 py-4">
    <!-- <div class="max-w-3xl">
      <h2 class="text-lg font-semibold text-white">{title}</h2>
      {#if description}
        <p class="mt-1 max-w-3xl text-sm text-slate-400">{description}</p>
      {/if}
    </div> -->

    <header
            class="flex flex-wrap items-end justify-between gap-4 border-b border-white/10 pb-5"
          >
    <div>
      <h2 class="mt-2 text-3xl font-semibold tracking-tight text-white">
        {title}
      </h2>
      <p class="mt-2 max-w-2xl text-sm text-slate-400">
        {description}
      </p>
    </div>
    </header>

    <div class="flex flex-wrap items-center justify-end gap-2">
      <slot name="actions" />
    </div>
  </div>

  <div class="p-5">
    <slot />
  </div>
</section>
