<script lang="ts">
  import { onMount } from "svelte";

  export let title: string;
  export let description = "";
  export let closeLabel = "取消";
  export let onClose: () => void;

  let dialog: HTMLDivElement;
  let restoreFocus: HTMLElement | null = null;

  onMount(() => {
    restoreFocus = document.activeElement as HTMLElement | null;
    requestAnimationFrame(() => {
      dialog.querySelector<HTMLElement>("[data-autofocus], button, input, select, textarea")?.focus();
    });
    return () => restoreFocus?.focus();
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = Array.from(
      dialog.querySelectorAll<HTMLElement>(
        'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
      )
    );
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }
</script>

<div class="modal-backdrop" role="presentation" on:mousedown={(event) => event.target === event.currentTarget && onClose()}>
  <div
    class="modal-dialog"
    bind:this={dialog}
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    on:keydown={handleKeydown}
  >
    <h2>{title}</h2>
    {#if description}<p>{description}</p>{/if}
    <slot></slot>
    <div class="modal-actions">
      <slot name="actions">
        <button class="button" type="button" on:click={onClose}>{closeLabel}</button>
      </slot>
    </div>
  </div>
</div>
