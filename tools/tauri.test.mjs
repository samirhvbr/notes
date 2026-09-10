import { test } from "node:test";
import assert from "node:assert/strict";
import { developmentArgs } from "./tauri.mjs";

test("desktop development gets the first version from the authority", () => {
  assert.deepEqual(developmentArgs(["dev", "--no-watch"], "# Version\n0.13.4\nPrevious: 0.13.3"),
    ["dev", "--no-watch", "--config", '{"version":"0.13.4"}']);
});
test("mobile development gets the same version", () => {
  for (const platform of ["ios", "android"]) {
    assert.deepEqual(developmentArgs([platform, "dev"], "1.2.3"),
      [platform, "dev", "--config", '{"version":"1.2.3"}']);
  }
});
test("application arguments remain after the separator", () => {
  assert.deepEqual(developmentArgs(["dev", "--", "--example"], "1.2.3"),
    ["dev", "--config", '{"version":"1.2.3"}', "--", "--example"]);
});
test("build and other commands retain their existing stamping behavior", () => {
  for (const args of [["build"], ["ios", "build"], ["info"], ["--help"]]) {
    assert.deepEqual(developmentArgs(args, "1.2.3"), args);
  }
});
test("development refuses a missing version instead of displaying a placeholder", () => {
  assert.throws(() => developmentArgs(["dev"], "missing"), /version.md/);
});
