import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ReceivedSyncShell } from "./app/ReceivedSync";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ReceivedSyncShell><App /></ReceivedSyncShell>
  </React.StrictMode>
);
