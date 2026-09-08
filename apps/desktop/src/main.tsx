import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ImportWizard } from "./ImportWizard";

const params = new URLSearchParams(window.location.search);
const root = (
  <React.StrictMode>
    {params.get("wizard") === "import" ? <ImportWizard /> : <App />}
  </React.StrictMode>
);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(root);
