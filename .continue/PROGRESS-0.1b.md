# PROGRESS — marco 0.1b

> Onde o 0.1b parou. Arquivo de retomada: se a sessão morrer, comece lendo isto,
> depois `docs/DECISIONS-0.1b.md` e o `git log`. Some quando o 0.1b fechar.
>
> Escopo do marco (`SCOPE_final.md` §17): rename, move, duplicate, delete
> (lixeira) · watcher + reconciliação no foco · UI de conflito · preview e split
> · busca/substituição no arquivo.

## Etapas, na ordem combinada

| # | Etapa | Estado |
|---|---|---|
| 0 | Dívida do 0.1a — link do `.continue/README.md`, alvo Windows por padrão, ENOSPC automatizado | **feito** — `0.7.5` |
| 1 | Fixtures do preview + suíte de asserção do `fixtures/xss/` | em curso |
| 2 | `notes-markdown` (C2): parse, sanitize, links, front matter | não começou |
| 3 | IR pelo §10 — HTML sanitizado no IPC, AST não | não começou |
| 4 | UI: Source / Preview / Split + tela "comparar" do conflito (C3) | não começou |
| 5 | Resto do escopo: rename/move/duplicate/delete, watcher, reconciliação | não começou |
| 6 | `docs/ACCEPTANCE-0.1b.md` | não começou |

## O que a etapa 0 resolveu

- **ENOSPC virou teste.** `tools/enospc.sh` monta um `tmpfs` de 1 MiB dentro de
  um user namespace não privilegiado e roda `crates/notes-core/tests/enospc.rs`
  lá dentro. Sem `sudo`, sem loopback, sem mount sobrando. Critério 5 do 0.1a
  passou de *partly met* para **met**. Razão em `DECISIONS-0.1b.md` D-03.
- **`tools/check.sh` roda o alvo Windows por padrão** e instala o target se
  faltar (D-01). Saída: `NOTES_NO_WINDOWS_CHECK=1`.
- **`.continue/README.md`** aponta para `../docs/ARCHITECTURE.md` (D-02).

## Regras desta rodada (do pedido do Samir, 07/09/2026)

- Opção mais simples compatível com o scope; escreva a seção que faltava no
  `docs/ARCHITECTURE.md`; siga sem perguntar.
- Parar só se: (a) mudar formato dos `.md` do usuário, (b) mudar escopo do 0.1b,
  (c) contradizer ADR `ACTIVE` ou `DECISIONS-0.1a.md`.
- Decisão minha vira linha em `docs/DECISIONS-0.1b.md`, com a alternativa
  descartada.
- Commit por etapa verde: `cargo build` + `clippy --all-targets` +
  `npm run build` + `cargo test` antes de cada um. `tools/check.sh` faz tudo.
- **Nada de `notes-index`, nada de busca global** — isso é 0.1c.
- `.continue/` intocada fora da etapa 0 e deste arquivo.
- ADR novo e `ARCHITECTURE.md` em `ACTIVE` só no commit final do marco.
