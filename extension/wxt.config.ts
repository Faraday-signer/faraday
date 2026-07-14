import { defineConfig } from "wxt";

export default defineConfig({
  srcDir: "src",
  entrypointsDir: "../entrypoints",
  modules: [],
  manifest: {
    name: "Faraday",
    description: "Air-gapped Solana QR relay wallet for browser dapps.",
    // Narrowest set that passes Chrome Web Store review. `tabs` is NOT
    // needed: chrome.tabs.create works without it (rejection ref:
    // "Purple Potassium" — excessive permissions). Camera access is not
    // a manifest permission; extension pages request it per-origin via
    // getUserMedia (always audio: false — we never touch the mic).
    permissions: ["storage", "sidePanel"],
    action: {},
    web_accessible_resources: [
      {
        resources: ["inpage.js"],
        matches: ["<all_urls>"]
      }
    ]
  }
});
