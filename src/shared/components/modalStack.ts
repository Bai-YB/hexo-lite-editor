const dialogs: HTMLElement[] = [];

export function registerModal(dialog: HTMLElement): () => void {
  dialogs.push(dialog);
  return () => {
    const index = dialogs.indexOf(dialog);
    if (index >= 0) dialogs.splice(index, 1);
  };
}

export function hasOpenModal(): boolean {
  return dialogs.length > 0;
}

export function isTopModal(dialog: HTMLElement): boolean {
  return dialogs.at(-1) === dialog;
}
