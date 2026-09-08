/**
 * A line diff, for the conflict comparison screen and nothing else.
 *
 * Written here rather than pulled in for the reason `notes-markdown` writes its
 * own slugs: it is sixty lines, it runs on two strings this application already
 * holds, and a dependency that changes how a diff aligns changes what a user
 * sees at the one moment they are deciding which version of their work to keep.
 *
 * It is **not** a merge. Nothing here writes anything: the screen shows the two
 * versions, and the three buttons under it are `conflict_resolve`.
 */

export type RowKind = "same" | "changed" | "mine" | "theirs";

export interface Row {
  kind: RowKind;
  /** The line from the buffer, when there is one. */
  mine: string | null;
  /** The line from disk, when there is one. */
  theirs: string | null;
  /** 1-based line numbers, for the gutters. */
  mineNo: number | null;
  theirsNo: number | null;
}

/**
 * Above this many cells the quadratic alignment is skipped and the differing
 * middle is shown as one replaced block.
 *
 * A 2 000 × 2 000 table is four million cells of `Uint32Array`, which is 16 MB
 * and a few milliseconds — fine. Ten times the lines is a hundred times the
 * table, which is not, and a note that large has a diff nobody reads line by
 * line anyway. Degrading loudly beats freezing the window.
 */
const MAX_CELLS = 4_000_000;

export interface Diff {
  rows: Row[];
  /** True when the middle was too large to align and is shown as one block. */
  coarse: boolean;
  changed: number;
}

export function diffLines(mine: string, theirs: string): Diff {
  const a = mine.split("\n");
  const b = theirs.split("\n");

  // Common prefix and suffix first: two versions of a note usually differ in
  // one paragraph, and trimming turns the quadratic step into nothing.
  let head = 0;
  while (head < a.length && head < b.length && a[head] === b[head]) head++;
  let tail = 0;
  while (
    tail < a.length - head &&
    tail < b.length - head &&
    a[a.length - 1 - tail] === b[b.length - 1 - tail]
  )
    tail++;

  const midA = a.slice(head, a.length - tail);
  const midB = b.slice(head, b.length - tail);

  const rows: Row[] = [];
  for (let i = 0; i < head; i++) {
    rows.push({ kind: "same", mine: a[i], theirs: b[i], mineNo: i + 1, theirsNo: i + 1 });
  }

  let coarse = false;
  if (midA.length * midB.length > MAX_CELLS) {
    coarse = true;
    for (let i = 0; i < Math.max(midA.length, midB.length); i++) {
      rows.push({
        kind: "changed",
        mine: midA[i] ?? null,
        theirs: midB[i] ?? null,
        mineNo: i < midA.length ? head + i + 1 : null,
        theirsNo: i < midB.length ? head + i + 1 : null,
      });
    }
  } else {
    for (const step of align(midA, midB)) {
      rows.push({
        kind: step.kind,
        mine: step.ai === null ? null : midA[step.ai],
        theirs: step.bi === null ? null : midB[step.bi],
        mineNo: step.ai === null ? null : head + step.ai + 1,
        theirsNo: step.bi === null ? null : head + step.bi + 1,
      });
    }
  }

  for (let i = 0; i < tail; i++) {
    const ai = a.length - tail + i;
    const bi = b.length - tail + i;
    rows.push({ kind: "same", mine: a[ai], theirs: b[bi], mineNo: ai + 1, theirsNo: bi + 1 });
  }

  return { rows, coarse, changed: rows.filter((r) => r.kind !== "same").length };
}

interface Step {
  kind: RowKind;
  ai: number | null;
  bi: number | null;
}

/** Longest common subsequence, then walked back into aligned rows. */
function align(a: string[], b: string[]): Step[] {
  const w = b.length + 1;
  const table = new Uint32Array((a.length + 1) * w);
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      table[i * w + j] =
        a[i] === b[j]
          ? table[(i + 1) * w + j + 1] + 1
          : Math.max(table[(i + 1) * w + j], table[i * w + j + 1]);
    }
  }

  const steps: Step[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      steps.push({ kind: "same", ai: i, bi: j });
      i++;
      j++;
    } else if (table[(i + 1) * w + j] >= table[i * w + j + 1]) {
      steps.push({ kind: "mine", ai: i, bi: null });
      i++;
    } else {
      steps.push({ kind: "theirs", ai: null, bi: j });
      j++;
    }
  }
  while (i < a.length) steps.push({ kind: "mine", ai: i++, bi: null });
  while (j < b.length) steps.push({ kind: "theirs", ai: null, bi: j++ });

  return pair(steps);
}

/**
 * A run of removals followed by a run of additions reads as a replacement.
 *
 * Without this the screen shows the old paragraph and the new one in separate
 * blocks and leaves the reader to work out that they are the same paragraph,
 * which is the whole question they are trying to answer.
 */
function pair(steps: Step[]): Step[] {
  const out: Step[] = [];
  let i = 0;
  while (i < steps.length) {
    if (steps[i].kind !== "mine") {
      out.push(steps[i++]);
      continue;
    }
    let m = i;
    while (m < steps.length && steps[m].kind === "mine") m++;
    let t = m;
    while (t < steps.length && steps[t].kind === "theirs") t++;

    const mines = steps.slice(i, m);
    const theirs = steps.slice(m, t);
    const n = Math.max(mines.length, theirs.length);
    for (let k = 0; k < n; k++) {
      const one = mines[k];
      const other = theirs[k];
      out.push({
        kind: one && other ? "changed" : one ? "mine" : "theirs",
        ai: one?.ai ?? null,
        bi: other?.bi ?? null,
      });
    }
    i = t;
  }
  return out;
}
