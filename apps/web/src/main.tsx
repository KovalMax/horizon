import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles.css";

createRoot(document.getElementById("root")!).render(<App />);

if (
  (import.meta.env.PROD || import.meta.env.VITE_ENABLE_LOCAL_PUSH === "true") &&
  "serviceWorker" in navigator
) {
  void navigator.serviceWorker.register("/sw.js").catch(() => {
    // Horizon still works online if the browser cannot install the worker.
  });
}
