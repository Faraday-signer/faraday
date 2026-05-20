import { defineConfig } from "wxt";

export default defineConfig({
  srcDir: "src",
  entrypointsDir: "../entrypoints",
  modules: [],
  manifest: {
    name: "Faraday",
    description: "Air-gapped Solana QR relay wallet for browser dapps.",
    permissions: ["storage", "tabs", "sidePanel"],
    action: {},
    web_accessible_resources: [
      {
        resources: ["inpage.js"],
        matches: ["<all_urls>"]
      }
    ]
  }
});
