#!/usr/bin/env bash
# Every text colour against every surface it can land on, at WCAG AA.
#
# WHY THIS IS A BUILD STEP AND NOT A REVIEW: `.continue/0.1d-interface.md` §5
# asks for AA against **each of the three dark levels**, and that is nine pairs
# before counting the accent — a number nobody re-checks by eye after changing
# one hex digit. The failure it prevents is quiet: a palette tweak that reads
# fine on the machine that made it and is unreadable on a laptop at an angle.
#
# It checks three different things and says which is which, because they are not
# the same kind of claim: 4.5:1 for text (WCAG 1.4.3), 3:1 for the focus ring
# (1.4.11), and — for the three dark surfaces and the divider — a step in sRGB
# rather than a contrast ratio, because near black the ratio formula's flare
# term swamps the difference a person can actually see. The reasoning is in the
# script, next to each floor.
#
#   tools/contrast.sh            # check
#   tools/contrast.sh --table    # print every ratio, then check
#
# Norm: docs/ACCEPTANCE-0.1d.md
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CSS="$ROOT/apps/notes-app/src/styles.css"

python3 - "$CSS" "${1:-}" <<'PY'
import re, sys

css_path, flag = sys.argv[1], sys.argv[2]
css = open(css_path).read()

# The tokens, read out of `:root` rather than restated here — a checker with its
# own copy of the palette checks its own copy.
root = css[css.index(":root {"):]
root = root[: root.index("\n}")]
tokens = dict(re.findall(r"--([\w-]+):\s*(#[0-9a-fA-F]{3,8})\s*;", root))

missing = [n for n in ("bg", "bg-raised", "bg-sunken", "fg", "fg-dim", "accent", "line",
                       "bad", "good", "warn", "selected", "disabled", "field") if n not in tokens]
if missing:
    print(f"contrast: these tokens are not simple hex in :root — {', '.join(missing)}")
    sys.exit(1)

# **A colour written outside `:root` is a colour this script cannot see.** That
# is the failure mode of a checker that reads tokens, so it is checked here
# rather than trusted: every hex below the token block has to become a token
# first, and then it is covered like the rest.
after_root = css[css.index(":root {"):]
after_root = after_root[after_root.index("\n}") :]
loose = re.findall(r"(?<![\w-])#[0-9a-fA-F]{3,8}\b", after_root)
if loose:
    print(
        "contrast: raw colours outside :root — "
        + ", ".join(sorted(set(loose)))
        + "\n  Give each a token in :root and use var(). A hex written inline is a"
        "\n  colour no contrast check can reach.",
        file=sys.stderr,
    )
    sys.exit(1)


def rgb(h):
    h = h.lstrip("#")
    if len(h) == 3:
        h = "".join(c * 2 for c in h)
    return tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))


def luminance(h):
    def lin(c):
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    r, g, b = (lin(c) for c in rgb(h))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = luminance(a), luminance(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


SURFACES = ["bg", "bg-raised", "bg-sunken"]

# WCAG 2.2, 1.4.3: 4.5:1 for body text.
TEXT = [
    ("fg", "body text"),
    ("fg-dim", "secondary text — counters, paths, tooltips; §5 says still AA"),
    ("accent", "links and the active label"),
    ("bad", "error text"),
    ("warn", "warning text"),
    ("good", "the saved state"),
]

# Surfaces a row can be *chosen* on, which then have to carry the text too. A
# selected tree row is the common case, and it is the one where a palette that
# only checked the three base levels goes wrong.
EXTRA_SURFACES = ["selected", "field"]

# WCAG 2.2, 1.4.11: 3:1 for what is needed to *identify a control or its state*.
# The focus ring is the clearest case — it is the only thing telling a keyboard
# user where they are.
#
# **`--line` is deliberately not in this list.** 1.4.11 covers visual
# information required to identify components, and excludes what is purely
# decorative. A rule between two panels that are already different surfaces
# identifies nothing on its own: remove it and the sidebar is still a sidebar.
# Holding it to 3:1 would mean a near-white hairline on a dark ground — brighter
# than the secondary text it sits beside, which is a worse interface and a
# misreading of the criterion. What it does have to be is *visible*, and that is
# checked below against this project's own floor rather than against a standard
# that does not apply.
FOCUS = [("accent", "the focus ring and the active-state marker")]

# Not WCAG, and **not a contrast ratio**.
#
# `.continue/0.1d-interface.md` §5: *"A diferença entre níveis tem que ser
# visível numa tela ruim."* A contrast ratio is the wrong instrument for that
# question. Its formula adds 0.05 to both luminances to model ambient flare on
# the screen, and near black that constant dominates: three surfaces a person
# can plainly tell apart score 1.05–1.13, and pushing any pair to the 3:1 a
# standard would ask for means one of them is no longer dark. The first version
# of this script set a 1.15 floor, and no palette that still reads as one tone
# of dark could reach it on the second step.
#
# What a bad panel actually loses is **code values near black** — it crushes
# them together. So the level check is a step in sRGB, where 8/255 is about
# where a cheap panel stops merging two greys, and it is measured on the
# channel that carries the most of the difference.
LEVEL_STEP = 8
LINE_STEP = 8


def srgb_step(a, b):
    """The widest per-channel distance, in 8-bit sRGB."""
    return max(
        abs(int(x * 255 + 0.5) - int(y * 255 + 0.5)) for x, y in zip(rgb(a), rgb(b))
    )


rows, failures = [], []


def check(fg, bg, need, why, kind):
    r = ratio(tokens[fg], tokens[bg])
    rows.append((fg, bg, r, need, kind))
    if r < need:
        failures.append((fg, bg, f"{r:.2f}:1", f"{need}:1", why))


def check_step(a, b, need, why, kind):
    s = srgb_step(tokens[a], tokens[b])
    rows.append((a, b, float(s), float(need), kind))
    if s < need:
        failures.append((a, b, f"{s}/255", f"{need}/255", why))


for fg, why in TEXT:
    for bg in SURFACES + EXTRA_SURFACES:
        check(fg, bg, 4.5, why, "AA text")

for fg, why in FOCUS:
    for bg in SURFACES:
        check(fg, bg, 3.0, why, "AA non-text")

for bg in SURFACES:
    check_step("line", bg, LINE_STEP, "a divider nobody can see is not a divider", "step")

# An inert control still has to be readable — §3's rule that a thing which is
# coming is shown, not hidden. It is not *text* in the AA sense, so 3:1.
for bg in SURFACES:
    check("disabled", bg, 3.0, "a disabled control that cannot be read is a smudge", "AA non-text")

for a, b in zip(SURFACES, SURFACES[1:]):
    check_step(a, b, LEVEL_STEP, "two surfaces a bad screen would show as one", "step")

if flag == "--table":
    print(f"{'foreground':<12} {'surface':<12} {'ratio':>7}  {'need':>5}  what")
    for fg, bg, r, need, kind in rows:
        mark = " " if r >= need else "✗"
        unit = "/255" if kind == "step" else ":1"
        print(f"{fg:<12} {bg:<12} {r:>7.2f}{unit:<5} {need:>5.2f} {mark} {kind}")
    print()

if failures:
    for fg, bg, got, need, why in failures:
        print(
            f"contrast: --{fg} against --{bg} is {got}, below {need} — {why}",
            file=sys.stderr,
        )
    print(
        f"\n{len(failures)} pair(s) under their floor. The palette is three levels of dark\n"
        f"and one accent (.continue/0.1d-interface.md §5); a level that cannot carry the\n"
        f"text on it is not a level.",
        file=sys.stderr,
    )
    sys.exit(1)

text = sum(1 for r in rows if r[4] == "AA text")
steps = sum(1 for r in rows if r[4] == "step")
print(
    f"contrast: {len(rows)} pairs — {text} at AA for text, 3 at AA for the focus "
    f"ring, {steps} surface steps wide enough to survive a bad panel"
)
PY
