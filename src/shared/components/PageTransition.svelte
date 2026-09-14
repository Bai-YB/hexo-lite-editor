<script lang="ts">
  import { cubicIn, quintOut } from "svelte/easing";

  export let pageKey: string;
  export let duration = 180;

  const reducedMotion = () => typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  $: enterDuration = Math.min(200, Math.max(0, duration));
  $: leaveDuration = Math.min(100, Math.max(0, duration - 80));

  function pageEnter(_node: Element, options: { duration: number }) {
    const reduce = reducedMotion();
    return {
      duration: reduce ? 0 : options.duration,
      easing: quintOut,
      css: (t: number) => `opacity: ${t}; transform: translateY(${reduce ? 0 : (1 - t) * 4}px); pointer-events: auto;`
    };
  }

  function pageLeave(_node: Element, options: { duration: number }) {
    return {
      duration: reducedMotion() ? 0 : options.duration,
      easing: cubicIn,
      css: (t: number) => `opacity: ${t}; pointer-events: none;`
    };
  }
</script>

<div
  class="page-transition"
  data-page-key={pageKey}
  in:pageEnter={{ duration: enterDuration }}
  out:pageLeave={{ duration: leaveDuration }}
>
  <slot />
</div>

<style>
  .page-transition {
    position: absolute;
    inset: 0;
    min-width: 0;
    min-height: 0;
    pointer-events: auto;
  }

</style>
