import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"
import ronMiddleware from "./dev/vite.ron-middleware"

const resolveFromRoot = (relativePath: string) =>
  decodeURIComponent(new URL(relativePath, import.meta.url).pathname)

// https://vitejs.dev/config/
export default defineConfig(({ command }) => ({
  plugins: [
    react({
      babel: {
        plugins: [["babel-plugin-react-compiler", {}]],
      },
    }),
    command === "serve" ? ronMiddleware() : undefined,
  ].filter(Boolean),
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
  publicDir: resolveFromRoot("../../assets"),
}))
