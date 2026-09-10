#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = new URL("../", import.meta.url);

export function developmentArgs(args, versionText) {
  const desktop = args[0] === "dev";
  const mobile = ["android", "ios"].includes(args[0]) && args[1] === "dev";
  if (!desktop && !mobile) return args;
  const version = versionText.match(/\d+\.\d+\.\d+/)?.[0];
  if (!version) throw new Error("version.md must contain a version (X.Y.Z)");
  // Before the separator: arguments after it belong to the application.
  const separator = args.indexOf("--");
  const end = separator < 0 ? args.length : separator;
  return [...args.slice(0, end), "--config", JSON.stringify({ version }), ...args.slice(end)];
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const require = createRequire(new URL("apps/notes-app/package.json", root));
  const cli = require.resolve("@tauri-apps/cli/tauri.js");
  const args = developmentArgs(process.argv.slice(2), readFileSync(new URL("version.md", root), "utf8"));
  const child = spawn(process.execPath, [cli, ...args], {
    cwd: fileURLToPath(new URL("apps/notes-app/", root)),
    stdio: "inherit",
  });
  for (const signal of ["SIGINT", "SIGTERM"]) {
    process.on(signal, () => child.kill(signal));
  }
  child.on("error", (error) => { console.error(error.message); process.exitCode = 1; });
  child.on("exit", (code, signal) => { process.exitCode = code ?? (signal === "SIGINT" ? 130 : 1); });
}
