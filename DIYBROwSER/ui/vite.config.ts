import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Port dev dikunci karena devUrl di src-tauri/tauri.conf.json menunjuk ke sini.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  build: {
    rollupOptions: {
      // Dua webview: chrome (`index.html`) dan panel melayang (`panel.html`).
      input: {
        utama: "index.html",
        panel: "panel.html",
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
