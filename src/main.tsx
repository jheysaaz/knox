import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import App from "./App";
import { ErrorBoundary } from "./components/error-boundary";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);

queueMicrotask(async () => {
  const win = getCurrentWindow();
  try {
    await win.show();
    await win.setFocus();
  } catch (err) {
    console.error("Failed to show window — missing core:window:allow-show permission?", err);
  }
  requestAnimationFrame(() => {
    invoke("log_window_shown");
  });
});
