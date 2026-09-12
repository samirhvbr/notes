import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { UpdaterShell } from "./app/Updater";
import { ReceivedSyncShell } from "./app/ReceivedSync";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <UpdaterShell><ReceivedSyncShell><App /></ReceivedSyncShell></UpdaterShell>
  </React.StrictMode>
);
