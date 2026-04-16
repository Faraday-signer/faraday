import React from "react";
import { createRoot } from "react-dom/client";

import { PopupApp } from "./popup-app";

const rootNode = document.getElementById("root");
if (!rootNode) {
  throw new Error("Missing popup root element.");
}

createRoot(rootNode).render(
  <React.StrictMode>
    <PopupApp />
  </React.StrictMode>
);
