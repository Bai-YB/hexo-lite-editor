<script lang="ts">
  import { cubicIn, cubicOut } from "svelte/easing";

  export let pageKey: string;
  export let duration = 160;

  const reduceMotion =
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  $: enterDuration = reduceMotion ? 0 : Math.min(200, Math.max(0, duration));
  $: leaveDuration = reduceMotion ? 0 : Math.min(120, Math.max(0, duration - 60));

  function pageEnter(_node: Element, options: { duration: number }) {
    return {
      duration: options.duration,
      easing: cubicOut,
      css: (t: number) => `opacity: ${t}; transform: translateY(${(1 - t) * 6}px); pointer-events: auto;`
    };
  }

  function pageLeave(_node: Element, options: { duration: number }) {
    return {
      duration: options.duration,
      easing: cubicIn,
      css: (t: number) => `opacity: ${t}; transform: translateY(${(1 - t) * -4}px); pointer-events: none;`
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
  }
</style>
