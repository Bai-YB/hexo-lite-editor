export interface PreviewAssetState {
  uploadId: string;
  localPreviewUrl: string;
  remoteUrl?: string;
  width?: number;
  height?: number;
  status: "local" | "uploading" | "remote" | "failed";
}

export class PreviewAssetRegistry {
  private assets = new Map<string, PreviewAssetState>();

  register(state: Omit<PreviewAssetState, "status"> & { status?: PreviewAssetState["status"] }): void {
    this.assets.set(state.uploadId, { ...state, status: state.status ?? "local" });
  }
  markUploading(uploadId: string): void { this.update(uploadId, { status: "uploading" }); }
  resolveRemote(uploadId: string, remoteUrl: string): void { this.update(uploadId, { status: "remote", remoteUrl }); }
  markFailed(uploadId: string): void { this.update(uploadId, { status: "failed" }); }
  remove(uploadId: string): void { this.assets.delete(uploadId); }
  get(uploadId: string): PreviewAssetState | undefined { const value = this.assets.get(uploadId); return value ? { ...value } : undefined; }
  clear(): void { this.assets.clear(); }

  private update(uploadId: string, patch: Partial<PreviewAssetState>): void {
    const value = this.assets.get(uploadId);
    if (!value) return;
    this.assets.set(uploadId, { ...value, ...patch });
  }
}

export function replacePreviewAssetInPlace(container: HTMLElement, uploadId: string, remoteUrl: string): HTMLImageElement | null {
  const image = [...container.querySelectorAll<HTMLImageElement>("img[data-upload-id]")].find((item) => item.dataset.uploadId === uploadId);
  if (!image) return null;
  const scrollTop = container.scrollTop;
  const width = image.getBoundingClientRect().width;
  const height = image.getBoundingClientRect().height;
  if (width) image.style.width = `${width}px`;
  if (height) image.style.height = `${height}px`;
  image.dataset.imageState = "remote";
  image.src = remoteUrl;
  container.scrollTop = scrollTop;
  return image;
}
