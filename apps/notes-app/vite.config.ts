import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri drives this dev server, so the port is fixed and failure to bind is an
// error rather than a silent shift to another port — a moved port shows up as a
// blank window, which is the hardest kind of spike failure to read.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Mobile devices load the dev server over the network, so it cannot bind to
    // localhost only. TAURI_DEV_HOST is set by `tauri android|ios dev`.
    host: process.env.TAURI_DEV_HOST || false,
    hmr: process.env.TAURI_DEV_HOST
      ? { protocol: "ws", host: process.env.TAURI_DEV_HOST, port: 1421 }
      : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    // Per file, because the store tests are pure logic and want no DOM at all,
    // while the menu tests are about focus — which only a DOM has. A file opts
    // in with `// @vitest-environment jsdom` at the top.
    environment: "node",
    globals: false,
    setupFiles: ["./vitest.setup.ts"],
  },
  build: {
    target: ["es2021", "chrome100", "safari15"],
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    // `oxc`, not `esbuild`: Vite 8 minifies with oxc and no longer ships
    // esbuild at all, so naming esbuild here asks for a package that is not
    // installed and fails the build in `renderChunk` — after a successful
    // transform, which is what makes it read like a plugin bug. The `target`
    // above is still what decides the syntax floor.
    minify: process.env.TAURI_ENV_DEBUG ? false : "oxc",
  },
});
