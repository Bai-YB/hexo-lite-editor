// Let the browser-only demo show the macOS layout on a Windows review machine.
const demoMacLayout = typeof window !== "undefined"
  && new URLSearchParams(window.location.search).get("demo") === "1"
  && new URLSearchParams(window.location.search).get("macLayout") === "1";

export const isMacOS = demoMacLayout || (typeof navigator !== "undefined"
  && /Macintosh|Mac OS X/.test(navigator.userAgent));

export function shortcutLabel(keys: string) {
  return isMacOS ? `⌘${keys}` : `Ctrl+${keys}`;
}
