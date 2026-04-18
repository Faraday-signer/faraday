import { defineConfig } from "wxt";

export default defineConfig({
  modules: [],
  manifest: {
    name: "Faraday",
    description: "Air-gapped Solana QR relay wallet for browser dapps.",
    permissions: ["storage", "tabs", "sidePanel"],
    action: {},
    host_permissions: ["<all_urls>"],
    web_accessible_resources: [
      {
        resources: ["inpage.js"],
        matches: ["<all_urls>"]
      }
    ]
  }
});
