<script lang="ts">
  import { onMount } from 'svelte';

  import '../app.css';
  import AppShell from '$lib/components/AppShell.svelte';
  import ErrorOverlay from '$lib/components/ErrorOverlay.svelte';
  import { describeError, type CapturedError } from '$lib/utils/errors';

  type AlertAction = {
    label: string;
    run: () => void;
  };

  type ActiveAlert = {
    error: CapturedError;
    source: 'boundary' | 'global';
    primaryAction: AlertAction;
    secondaryAction: AlertAction | null;
  };

  let activeAlert: ActiveAlert | null = null;

  function clearAlert() {
    activeAlert = null;
  }

  function setAlert(next: ActiveAlert) {
    if (activeAlert?.source === 'boundary' && next.source === 'global') {
      return;
    }

    activeAlert = next;
  }

  function showBoundaryAlert(error: unknown, reset: () => void) {
    setAlert({
      error: describeError(error, 'Application screen crashed'),
      source: 'boundary',
      primaryAction: {
        label: 'Retry view',
        run: reset,
      },
      secondaryAction: {
        label: 'Reload app',
        run: () => window.location.reload(),
      },
    });
  }

  function showGlobalAlert(error: unknown, title: string) {
    setAlert({
      error: describeError(error, title),
      source: 'global',
      primaryAction: {
        label: 'Reload app',
        run: () => window.location.reload(),
      },
      secondaryAction: {
        label: 'Dismiss',
        run: clearAlert,
      },
    });
  }

  function runPrimaryAction() {
    const action = activeAlert?.primaryAction;
    clearAlert();
    action?.run();
  }

  function runSecondaryAction() {
    const action = activeAlert?.secondaryAction;
    clearAlert();
    action?.run();
  }

  onMount(() => {
    const handleWindowError = (event: ErrorEvent) => {
      showGlobalAlert(event.error ?? event.message, 'Unexpected browser error');
    };

    const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
      showGlobalAlert(event.reason, 'Unhandled promise rejection');
    };

    window.addEventListener('error', handleWindowError);
    window.addEventListener('unhandledrejection', handleUnhandledRejection);

    return () => {
      window.removeEventListener('error', handleWindowError);
      window.removeEventListener('unhandledrejection', handleUnhandledRejection);
    };
  });
</script>

<svelte:boundary onerror={showBoundaryAlert}>
  <AppShell>
    <slot />
  </AppShell>

  {#snippet failed(error, reset)}
    <div class="sr-only">Application screen crashed.</div>
  {/snippet}
</svelte:boundary>

{#if activeAlert}
  <ErrorOverlay
    error={activeAlert.error}
    primaryLabel={activeAlert.primaryAction.label}
    secondaryLabel={activeAlert.secondaryAction?.label ?? null}
    showSecondary={activeAlert.secondaryAction !== null}
    onPrimary={runPrimaryAction}
    onSecondary={activeAlert.secondaryAction ? runSecondaryAction : null}
  />
{/if}
