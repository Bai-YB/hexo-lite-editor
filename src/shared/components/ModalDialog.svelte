<script lang="ts">
  import { onMount } from "svelte";
  import { cubicIn, cubicOut } from "svelte/easing";
  import { fade } from "svelte/transition";

  export let title: string;
  export let description = "";
  export let closeLabel = "取消";
  export let onClose: () => void;

  let dialog: HTMLDivElement;
  let restoreFocus: HTMLElement | null = null;

  function dialogTransition(_node: Element, options: { duration: number }) {
    return {
      duration: options.duration,
      easing: options.duration > 120 ? cubicOut : cubicIn,
      css: (t: number) => `opacity:${t};transform:translateY(${(1 - t) * 4}px) scale(${0.97 + t * 0.03})`
    };
  }

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
    if (
      event.key === "Enter"
      && !event.ctrlKey
      && !event.metaKey
      && !event.shiftKey
      && event.target instanceof HTMLInputElement
    ) {
      const submit = dialog.querySelector<HTMLButtonElement>(".modal-actions .button.primary:not(:disabled)");
      if (submit) {
        event.preventDefault();
        submit.click();
      }
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = Array.from(
      dialog.querySelectorAll<HTMLElement>(
        'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
      )
    );
    if (!focusable.length) {
      event.preventDefault();
      dialog.focus();
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (!dialog.contains(document.activeElement)) {
      event.preventDefault();
      first.focus();
    } else if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div
  class="modal-backdrop"
  role="presentation"
  transition:fade={{ duration: 120 }}
  on:mousedown={(event) => event.target === event.currentTarget && onClose()}
>
  <div
    class="modal-dialog"
    bind:this={dialog}
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    in:dialogTransition={{ duration: 160 }}
    out:dialogTransition={{ duration: 100 }}
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
