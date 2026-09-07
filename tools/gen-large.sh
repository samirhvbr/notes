#!/usr/bin/env bash
# Generate `fixtures/large/` — the performance corpus. Never committed
# (`.gitignore`, ADR-011); regenerated in CI and locally on demand.
#
# Deterministic: the same SEED produces byte-identical output on any machine, so
# a timing regression is a change in the code and not in the corpus.
#
#   tools/gen-large.sh            10000 notes, ~200 MB
#   COUNT=1000 tools/gen-large.sh smaller, for a quick local loop
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${OUT:-$ROOT/fixtures/large}"
COUNT="${COUNT:-10000}"
SEED="${SEED:-notes-0.1a}"

if [ -e "$OUT" ]; then
  echo "removing existing $OUT" >&2
  rm -rf "$OUT"
fi

python3 - "$OUT" "$COUNT" "$SEED" <<'PY'
import hashlib, pathlib, sys

out, count, seed = pathlib.Path(sys.argv[1]), int(sys.argv[2]), sys.argv[3]

WORDS = ("servidor firewall backup rede cliente projeto reuniao nota tarefa deploy "
         "container volume indice consulta metrica alerta incidente runbook politica "
         "chave rotacao inventario topologia latencia janela retencao roteador switch "
         "cluster snapshot replica particao throughput jitter").split()

def stream(s, n):
    out, buf = bytearray(), b""
    while len(out) < n:
        buf = hashlib.blake2b(s.encode() + buf, digest_size=64).digest()
        out.extend(buf)
    return out[:n]

def para(s, n):
    return " ".join(WORDS[b % len(WORDS)] for b in stream(s, n))

# A wide, shallow-ish tree: 100 dirs of 2 levels, so listing one level stays
# cheap and the tree is realistic rather than one flat directory.
dirs = [f"area-{i:02d}/sub-{j:02d}" for i in range(10) for j in range(10)]
for d in dirs:
    (out / d).mkdir(parents=True, exist_ok=True)

total = 0
for i in range(count):
    d = dirs[i % len(dirs)]
    p = out / d / f"nota-{i:05d}.md"
    s = f"{seed}/{i}"
    body = [f"# {para(s + 'h', 5).title()}", ""]
    for k in range(3):                      # ~20 KB per note → ~200 MB at 10k
        body.append(para(f"{s}/{k}", 850))
        body.append("")
    data = ("\n".join(body)).encode("utf-8")
    p.write_bytes(data)
    total += len(data)

print(f"{count} notes, {total / 1024 / 1024:.1f} MiB, in {out}")
PY
