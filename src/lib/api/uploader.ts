import { command } from "./tauri";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ImageBedListResult, UploadResult } from "$lib/types/uploader";

interface RawUploadResult {
  url: string;
  markdown: string;
  file_path: string;
}

interface RawImageBedListResult {
  files: Array<{
    id: string;
    name: string;
    url: string;
    file_name: string;
    file_type: string;
    file_size: string;
    created_at: string;
    channel: string;
    raw: unknown;
  }>;
  directories: string[];
  total_count: number;
  returned_count: number;
}

export async function copyImageToProject(projectPath: string, imagePath: string): Promise<UploadResult> {
  const result = await command<RawUploadResult>("copy_image_to_project", { projectPath, imagePath });
  return {
    url: result.url,
    markdown: result.markdown,
    filePath: result.file_path
  };
}

export async function saveClipboardImageToProject(
  projectPath: string,
  fileName: string | undefined,
  mimeType: string | undefined,
  data: number[]
): Promise<UploadResult> {
  const result = await command<RawUploadResult>("save_clipboard_image_to_project", {
    projectPath,
    fileName,
    mimeType,
    data
  });
  return {
    url: result.url,
    markdown: result.markdown,
    filePath: result.file_path
  };
}

export async function uploadImagePathToCloudflareImgBed(
  apiUrl: string,
  token: string | undefined,
  imagePath: string
): Promise<UploadResult> {
  const result = await command<RawUploadResult>("upload_image_path_to_cloudflare_imgbed", {
    apiUrl,
    token,
    imagePath
  });
  return {
    url: result.url,
    markdown: result.markdown,
    filePath: result.file_path
  };
}

export async function uploadClipboardImageToCloudflareImgBed(
  apiUrl: string,
  token: string | undefined,
  fileName: string | undefined,
  mimeType: string | undefined,
  data: number[]
): Promise<UploadResult> {
  const result = await command<RawUploadResult>("upload_clipboard_image_to_cloudflare_imgbed", {
    apiUrl,
    token,
    fileName,
    mimeType,
    data
  });
  return {
    url: result.url,
    markdown: result.markdown,
    filePath: result.file_path
  };
}

export async function listCloudflareImgBedImages(
  apiUrl: string,
  token: string | undefined,
  start = 0,
  count = 50,
  search = "",
  dir = ""
): Promise<ImageBedListResult> {
  const result = await command<RawImageBedListResult>("list_cloudflare_imgbed_images", {
    apiUrl,
    token,
    start,
    count,
    search,
    dir
  });
  return {
    files: result.files.map((file) => ({
      id: file.id,
      name: file.name,
      url: file.url,
      fileName: file.file_name,
      fileType: file.file_type,
      fileSize: file.file_size,
      createdAt: file.created_at,
      channel: file.channel,
      raw: file.raw
    })),
    directories: result.directories,
    totalCount: result.total_count,
    returnedCount: result.returned_count
  };
}

export async function deleteCloudflareImgBedImage(
  apiUrl: string,
  token: string | undefined,
  fileId: string
): Promise<void> {
  await command<void>("delete_cloudflare_imgbed_image", { apiUrl, token, fileId });
}

export function cloudflareImgBedAdminUrl(apiUrl: string): string {
  const trimmed = apiUrl.trim();
  if (!trimmed) throw new Error("请先填写 CloudFlare-ImgBed API 地址。");
  const parsed = new URL(trimmed);
  return `${parsed.origin}/admin`;
}

export async function openCloudflareImgBedAdmin(apiUrl: string): Promise<void> {
  await openUrl(cloudflareImgBedAdminUrl(apiUrl));
}
