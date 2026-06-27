export interface Post {
  id: string;
  title: string;
  fileName: string;
  filePath: string;
  content?: string;
  rawFrontMatter?: Record<string, unknown>;
  date?: string;
  cover?: string;
  topImg?: string;
  banner?: string;
  thumbnail?: string;
  indexImg?: string;
  description?: string;
  tags: string[];
  categories: string[];
  createdAt?: string;
  updatedAt?: string;
  isDraft: boolean;
  isDirty?: boolean;
}

export interface CreatePostInput {
  title: string;
  fileName: string;
  categories: string[];
  tags: string[];
  date: string;
}
