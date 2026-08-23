export type PreviewMode = "quick" | "theme";

export function createDebouncedRefresh(callback: () => void, delay = 450) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  return {
    schedule() { clearTimeout(timer); timer = setTimeout(callback, delay); },
    cancel() { clearTimeout(timer); timer = undefined; }
  };
}
