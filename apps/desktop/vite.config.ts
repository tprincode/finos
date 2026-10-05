import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // The workspace packages (@finos/ui-components, @finos/app-contracts) are served from source,
  // not pre-bundled, so their `import … from "react"` is rewritten to whatever optimizer hash is
  // current. Adding one dependency edge inside a workspace package used to re-run the optimizer
  // mid-session and leave the page holding react.js?v=OLD while the package loaded
  // react.js?v=NEW. Two React instances means the hook dispatcher is null and the app dies at
  // boot with "Cannot read properties of null (reading 'useState')". dedupe forces one copy and
  // include pins the entry points into the first optimizer pass so there is no second pass.
  resolve: {
    dedupe: ["react", "react-dom"],
  },
  optimizeDeps: {
    include: [
      "react",
      "react-dom",
      "react-dom/client",
      "react/jsx-runtime",
      "react/jsx-dev-runtime",
    ],
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
