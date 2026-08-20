import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig(async () => ({ base: "./",
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));