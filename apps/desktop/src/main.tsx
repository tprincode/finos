import React, { Component, type ErrorInfo, type ReactNode } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import App from "./App";
import { ImportWizard } from "./ImportWizard";

function showBootError(message: string): void {
  const el = document.getElementById("root");
  if (!el) return;
  el.innerHTML = "";
  const pre = document.createElement("pre");
  pre.setAttribute("role", "alert");
  pre.style.cssText =
    "margin:1rem;padding:1rem;white-space:pre-wrap;font:14px/1.4 Consolas,monospace;background:#2a1010;color:#f6d7d7";
  pre.textContent = `finos failed to start:\n\n${message}`;
  el.appendChild(pre);
}

class BootErrorBoundary extends Component<
  { children: ReactNode },
  { error: string | null }
> {
  state = { error: null as string | null };

  static getDerivedStateFromError(error: Error): { error: string } {
    return { error: error?.stack || error?.message || String(error) };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    const text = `${error?.stack || error?.message || String(error)}\n${info.componentStack || ""}`;
    console.error("finos BootErrorBoundary", text);
    // A cold Vite start can serve two React copies for one paint. The next load
    // uses the finished optimizer graph. One reload, then show the error.
    if (
      text.includes("useState") &&
      sessionStorage.getItem("finos-react-reload") !== "1"
    ) {
      sessionStorage.setItem("finos-react-reload", "1");
      window.location.reload();
      return;
    }
    this.setState({ error: text });
    void invoke("page_loaded", { screen: "Home failed", detail: text }).catch(() => {});
  }

  render(): ReactNode {
    if (this.state.error) {
      return (
        <pre role="alert" style={{ margin: "1rem", padding: "1rem" }}>
          {`finos render error:\n\n${this.state.error}`}
        </pre>
      );
    }
    return this.props.children;
  }
}

window.addEventListener("error", (ev) => {
  const root = document.getElementById("root");
  if (root && root.childElementCount === 0) {
    showBootError(ev.message || String(ev.error || "unknown error"));
  }
});
window.addEventListener("unhandledrejection", (ev) => {
  const root = document.getElementById("root");
  if (root && root.childElementCount === 0) {
    showBootError(String(ev.reason || "unhandled rejection"));
  }
});

const params = new URLSearchParams(window.location.search);
const tree = (
  <React.StrictMode>
    <BootErrorBoundary>
      {params.get("wizard") === "import" ? <ImportWizard /> : <App />}
    </BootErrorBoundary>
  </React.StrictMode>
);

const mount = document.getElementById("root") as HTMLElement;
try {
  ReactDOM.createRoot(mount).render(tree);
} catch (err: unknown) {
  showBootError(String(err));
}
