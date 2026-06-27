import { writable } from "svelte/store";
import type { Post } from "$lib/types/post";

export const posts = writable<Post[]>([]);
export const activePost = writable<Post | null>(null);
