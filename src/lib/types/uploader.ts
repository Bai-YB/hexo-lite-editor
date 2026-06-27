export interface UploadResult {
  url: string;
  markdown: string;
  filePath: string;
}

export interface ImageUploader {
  id: string;
  name: string;
  type: "custom" | "smms" | "local" | "cloudflare-imgbed";
}

export interface ImageBedItem {
  id: string;
  name: string;
  url: string;
  fileName: string;
  fileType: string;
  fileSize: string;
  createdAt: string;
  channel: string;
  raw: unknown;
}

export interface ImageBedListResult {
  files: ImageBedItem[];
  directories: string[];
  totalCount: number;
  returnedCount: number;
}
