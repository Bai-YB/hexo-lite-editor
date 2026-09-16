<script lang="ts">
  import { cubicIn, quintOut } from "svelte/easing";

  export let pageKey: string;
  export let duration = 180;
  let transitionState: "active" | "leaving" = "active";

  const reducedMotion = () => typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  $: enterDuration = Math.min(200, Math.max(0, duration));
  $: leaveDuration = Math.min(100, Math.max(0, duration - 80));

  function pageEnter(_node: Element, options: { duration: number }) {
    const reduce = reducedMotion();
    return {
      duration: reduce ? 0 : options.duration,
      easing: quintOut,
      // Keep interactive controls at their final coordinates throughout the
      // fade. A translated absolute page can pass Playwright's stability
      // check and still move between hit testing and event delivery on a busy
      // WebView, making its workspace background intercept the click.
      css: (t: number) => `opacity: ${t}; pointer-events: auto;`
    };
  }

  function pageLeave(_node: HTMLElement, options: { duration: number }) {
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
  data-transition-state={transitionState}
  data-enter-duration-ms={enterDuration}
  data-leave-duration-ms={leaveDuration}
  in:pageEnter={{ duration: enterDuration }}
  out:pageLeave={{ duration: leaveDuration }}
  on:introstart={() => (transitionState = "active")}
  on:outrostart={() => (transitionState = "leaving")}
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

  /* This class is also cleared when Svelte reverses an interrupted outro.
     An imperative inline style would survive that reversal and leave the
     returned page visible but permanently unable to receive input. */
  .page-transition[data-transition-state="leaving"] {
    pointer-events: none;
  }

</style>
