import { writable } from "svelte/store";
import type { HexoProject } from "$lib/types/project";

export const currentProject = writable<HexoProject | null>(null);
