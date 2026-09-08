import { describe, expect, it } from "vitest";
import { diffLines } from "./diff";

const text = (rows: ReturnType<typeof diffLines>["rows"], side: "mine" | "theirs") =>
  rows.map((r) => r[side]).filter((l) => l !== null);

describe("diffLines", () => {
  it("shows two identical notes as entirely unchanged", () => {
    const d = diffLines("a\nb\nc\n", "a\nb\nc\n");
    expect(d.changed).toBe(0);
    expect(d.rows.every((r) => r.kind === "same")).toBe(true);
  });

  it("aligns a changed line against the line it replaced", () => {
    const d = diffLines("a\nMEU\nc", "a\nDELES\nc");
    expect(d.rows.map((r) => r.kind)).toEqual(["same", "changed", "same"]);
    expect(d.rows[1].mine).toBe("MEU");
    expect(d.rows[1].theirs).toBe("DELES");
  });

  it("keeps an added line on its own side, with nothing opposite it", () => {
    const d = diffLines("a\nb\nc", "a\nc");
    const added = d.rows.find((r) => r.kind === "mine");
    expect(added?.mine).toBe("b");
    expect(added?.theirs).toBeNull();
  });

  it("never loses a line from either version", () => {
    const mine = "um\ndois\ntres\nquatro";
    const theirs = "um\nDOIS\ntres\ncinco\nseis";
    const d = diffLines(mine, theirs);
    expect(text(d.rows, "mine").join("\n")).toBe(mine);
    expect(text(d.rows, "theirs").join("\n")).toBe(theirs);
  });

  it("numbers the gutters from one, per side", () => {
    const d = diffLines("a\nb", "a\nb");
    expect(d.rows.map((r) => r.mineNo)).toEqual([1, 2]);
    expect(d.rows.map((r) => r.theirsNo)).toEqual([1, 2]);
  });

  it("reports a replaced block rather than freezing on two large notes", () => {
    // Past MAX_CELLS the alignment is skipped on purpose. Degrading loudly
    // beats a window that stops responding at the moment a user is deciding
    // which version of their work to keep.
    const mine = Array.from({ length: 2100 }, (_, i) => `mine ${i}`).join("\n");
    const theirs = Array.from({ length: 2100 }, (_, i) => `theirs ${i}`).join("\n");
    const d = diffLines(mine, theirs);
    expect(d.coarse).toBe(true);
    expect(text(d.rows, "mine").join("\n")).toBe(mine);
    expect(text(d.rows, "theirs").join("\n")).toBe(theirs);
  });

  it("trims a common prefix and suffix, so a one-line change stays cheap", () => {
    const head = Array.from({ length: 3000 }, (_, i) => `linha ${i}`).join("\n");
    const d = diffLines(`${head}\nMEU\n${head}`, `${head}\nDELES\n${head}`);
    expect(d.coarse).toBe(false);
    expect(d.changed).toBe(1);
  });

  it("treats an empty note as one empty line rather than as nothing", () => {
    const d = diffLines("", "algo");
    expect(d.rows.length).toBeGreaterThan(0);
    expect(text(d.rows, "theirs")).toEqual(["algo"]);
  });
});
