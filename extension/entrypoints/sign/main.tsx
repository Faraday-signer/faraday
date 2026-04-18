import React from "react";
import { createRoot } from "react-dom/client";

import { SignApp } from "./sign-app";

const rootNode = document.getElementById("root");
if (!rootNode) {
  throw new Error("Missing sign root element.");
}

createRoot(rootNode).render(
  <React.StrictMode>
    <SignApp />
  </React.StrictMode>
);
