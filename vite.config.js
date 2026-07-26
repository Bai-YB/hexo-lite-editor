import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],
  build: {
    rollupOptions: {
      output: {
        /** @param {string} id */
        manualChunks(id) {
          if (id.includes("node_modules/@codemirror/lang-markdown")) return "editor-markdown";
          if (
            id.includes("node_modules/@codemirror/state") ||
            id.includes("node_modules/@codemirror/view")
          ) return "editor-runtime";
          if (
            id.includes("node_modules/@codemirror/language") ||
            id.includes("node_modules/@lezer")
          ) return "editor-language";
          if (
            id.includes("node_modules/@codemirror/commands") ||
            id.includes("node_modules/@codemirror/search")
          ) return "editor-actions";
          if (
            id.includes("node_modules/markdown-it") ||
            id.includes("node_modules/dompurify") ||
            id.includes("node_modules/linkify-it") ||
            id.includes("node_modules/mdurl")
          ) {
            return "markdown-preview";
          }
        }
      }
    }
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts", "scripts/**/*.test.mjs"],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
