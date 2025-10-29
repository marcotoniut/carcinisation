import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"

const resolveFromRoot = (relativePath: string) =>
  decodeURIComponent(new URL(relativePath, import.meta.url).pathname)

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react({
      babel: {
        plugins: [["babel-plugin-react-compiler", {}]],
      },
    }),
  ],
  resolve: {
    alias: {
      "@": resolveFromRoot("./src"),
      "@/types": resolveFromRoot("./src/types"),
      "@/components": resolveFromRoot("./src/components"),
      "@/api": resolveFromRoot("./src/api"),
      "@/state": resolveFromRoot("./src/state"),
      "@/hooks": resolveFromRoot("./src/hooks"),
      "@/utils": resolveFromRoot("./src/utils"),
    },
  },
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: "http://localhost:5174",
        changeOrigin: true,
      },
      "/ws": {
        target: "ws://localhost:5174",
        ws: true,
      },
    },
  },
})
