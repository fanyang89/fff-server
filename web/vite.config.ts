import path from "node:path"
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import tailwindcss from "@tailwindcss/vite"

// https://vite.dev/config/
export default defineConfig({
  // Relative asset URLs so the built SPA works under any path prefix when
  // combined with a runtime `<base href>` injected by the server. Absolute
  // base ("/") would break under /search because the browser would resolve
  // /assets/... against the origin root, not the prefix.
  base: "./",
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8787",
      "/swagger-ui": "http://127.0.0.1:8787",
      "/openapi.json": "http://127.0.0.1:8787",
    },
  },
})
