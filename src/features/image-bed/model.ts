import type { RemoteAssetKind } from "$shared/types/app";

export function shouldLoadCloudflare(input: {
  sessionReady: boolean;
  credentialReady: boolean;
  credentialConfigured: boolean;
  apiUrl: string;
}) {
  return input.sessionReady
    && input.credentialReady
    && input.credentialConfigured
    && input.apiUrl.trim().length > 0;
}

export function nextLightboxIndex(current: number, delta: number, count: number) {
  if (count <= 0) return -1;
  return (current + delta + count) % count;
}

export function assetKindLabel(kind: RemoteAssetKind) {
  return ({
    folder: "文件夹",
    image: "图片",
    archive: "压缩包",
    document: "文档",
    audio: "音频",
    video: "视频",
    file: "文件"
  } as const)[kind];
}
