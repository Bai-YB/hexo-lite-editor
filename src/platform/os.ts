export const isMacOS = typeof navigator !== "undefined"
  && /Macintosh|Mac OS X/.test(navigator.userAgent);

export function shortcutLabel(keys: string) {
  return isMacOS ? `⌘${keys}` : `Ctrl+${keys}`;
}
