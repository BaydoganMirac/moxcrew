import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { host: "127.0.0.1", port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/.tooling/**", "**/target/**", "**/dist/**"] } },
  preview: { host: "127.0.0.1" },
  build: { target: "safari14", outDir: "dist", sourcemap: false },
});
