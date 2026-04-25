import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { nodePolyfills } from "vite-plugin-node-polyfills";
import path from "node:path";

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    // @solana/web3.js + @sqds/multisig assume Buffer / process in scope.
    // Browsers don't have them; this shim injects what they need.
    nodePolyfills({ include: ["buffer", "process"], globals: { Buffer: true, process: true } }),
  ],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src")
    }
  },
  server: {
    port: 4175,
    strictPort: true
  }
});
