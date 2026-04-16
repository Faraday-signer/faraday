import React from "react";
import { createRoot } from "react-dom/client";

import "../../src/styles/global.css";
import { SidePanelApp } from "./sidepanel-app";

const rootNode = document.getElementById("root");
if (!rootNode) {
  throw new Error("Missing side panel root element.");
}

createRoot(rootNode).render(
  <React.StrictMode>
    <SidePanelApp />
  </React.StrictMode>
);
