import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  // GitHub Pages serves the editor under /vectro/; the Pages workflow sets VITE_BASE.
  base: process.env.VITE_BASE ?? "/",
  plugins: [svelte()],
  server: { port: 5173, strictPort: true },
});
