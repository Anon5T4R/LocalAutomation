import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Lição da suíte: uma única cópia do React (senão hooks quebram).
  resolve: {
    dedupe: ["react", "react-dom"],
  },

  // Opções do Vite ajustadas pro Tauri (só em `tauri dev`/`tauri build`).
  clearScreen: false,
  server: {
    // Porta única do LocalAutomation na suíte (LocalFeed=1466, este=1472 —
    // 1468/1470 ficaram livres, ver plano). devUrl e esta porta têm que bater.
    port: 1472,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1473,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
