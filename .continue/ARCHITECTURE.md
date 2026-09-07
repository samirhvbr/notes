# ARCHITECTURE — Notes

Documento **v0.1 — PROPOSTA, aguardando revisão**. 2026-09-07.

Fecha as decisões que o `SCOPE_final.md` §20 delega a este arquivo, **na ordem em
que o marco 0.1a precisa delas**. Cada seção traz as alternativas consideradas e
o custo de cada uma, porque decisão sem custo escrito não foi decidida.

**Cobertura desta versão:** §20.1 (schema do estado persistente), §20.2 (contrato
dos commands e modelo de erros), §20.6 (mapeamento de `Caps`).
**Fora desta versão**, porque não travam o 0.1a: §20.3 (IR do markdown — o
preview é 0.1b), §20.4 (lock entre processos — o consumidor é o `notes-mcp`,
0.3), §20.5 (`.notes/` portável), §20.7 (distribuição).

No fim há **[§5 — o que preciso que você confirme](#5-o-que-preciso-que-voce-confirme)**:
quatro pontos onde eu escolhi e a escolha é sua.

---

## 1. Estado persistente (§20.1)

### 1.1 Onde fica

O `SCOPE §6.1` define `<app>/workspaces/<WorkspaceId>/{registry.json, drafts/,
conflicts/, index.db, session.json}` e diz que o `WorkspaceId` é "chaveado pelo
path canônico da raiz".

**Falta um arquivo nessa lista, e ele é global.** "Persistir o último workspace"
(escopo do 0.1a) e "chavear por path canônico" são dados que não moram dentro de
`workspaces/<WorkspaceId>/` — para saber qual `WorkspaceId` corresponde a uma
pasta você precisa do índice antes de ter o id. Proposta:

```
<app_data>/
├── workspaces.json              índice global: path canônico → WorkspaceId
└── workspaces/
    └── <WorkspaceId>/
        ├── registry.json        identidade das notas
        ├── session.json         estado de UI deste workspace
        ├── drafts/              buffers suspensos            (0.1a)
        ├── conflicts/           versões preteridas           (schema aqui, escrita no 0.1b)
        └── index.db             derivado, descartável        (0.2)
```

`<app_data>` é o diretório por SO do Tauri (`~/.local/share/<id>` no Linux,
`~/Library/Application Support/<id>` no macOS, `%APPDATA%\<id>` no Windows).
**Depende do bundle identifier, que ainda não foi escolhido** — ver §5.

`index.db` fica aqui e não na pasta do usuário, conforme §6.1. Motivo já escrito
lá: banco SQLite ativo copiado por Dropbox/iCloud no meio de uma transação vira
cópia inconsistente.

### 1.2 Regras que valem para todo arquivo de estado

1. **Todo arquivo carrega `schema` (inteiro).** Sem exceção, desde o primeiro.
2. **`schema` maior que o conhecido → abre somente leitura e avisa.** Uma versão
   mais nova do app tocou este estado; sobrescrever seria destruir o que ela
   gravou. Não é erro do usuário e a mensagem diz isso.
3. **Migração faz cópia antes.** `registry.json` → `registry.json.bak-<schema>`,
   e só então grava a versão nova (`SCOPE §16`: versionamento, cópia prévia e
   recuperação).
4. **Toda gravação é atômica** — tmp no mesmo diretório, fsync, rename. O mesmo
   procedimento do §7.4, porque estado operacional truncado custa o mesmo que
   nota truncada.
5. **Nada aqui é reconstruível a partir das notas**, exceto `index.db`. É a
   categoria "operacional" do §6.1 e não entra em nenhuma limpeza automática.

### 1.3 `workspaces.json`

```jsonc
{
  "schema": 1,
  "last_workspace": "0193f2c1-...",        // null quando nenhum
  "workspaces": [
    {
      "id": "0193f2c1-...",
      "root": "/home/samir/Documents/notes",   // path canônico no momento do registro
      "label": "notes",                         // basename; só para exibir
      "last_opened": "2026-09-07T19:40:11Z"
    }
  ]
}
```

Abrir uma pasta: canonicaliza, procura por `root`, reusa o id se achar, cria se
não. Pasta movida não é encontrada por `root` — o app **oferece reconectar**
(§6.2) apresentando os workspaces cujo `root` não existe mais; recusar cria um
workspace novo. Nada disso é automático.

### 1.4 `registry.json` — e por que ele existe já no 0.1a

O `NoteId` só tem consumidor visível no 0.1b (rename que não reseta a aba) e no
0.6 (sync). A pergunta legítima é se o registro pode esperar.

**Não pode, por causa do rascunho.** Um buffer suspenso precisa saber a que nota
ele pertence. Se a chave for o `RelPath` e o arquivo for renomeado por fora
enquanto o rascunho está suspenso — exatamente a situação que produz rascunhos —
o rascunho vira órfão e o critério de aceite "buffer recuperável ao reabrir"
falha no caso que mais importa. Chaveando por `NoteId`, a correlação do §6.2
reencontra a nota.

```jsonc
{
  "schema": 1,
  "workspace_id": "0193f2c1-...",
  "case_sensitive_root": true,      // detectado uma vez, no registro; governa CompareKey
  "notes": {
    "0193f2d4-...": {                          // NoteId
      "rel_path": "infra/servidor.md",         // como está no disco, nunca reescrito
      "compare_key": "infra/servidor.md",      // NFC + case-fold quando o FS não distingue
      "size": 4213,
      "mtime": "2026-09-07T18:22:04.512Z",
      "content_hash": "b3:9f2a...",            // blake3, prefixado pelo algoritmo
      "native_id": "dev:2049/ino:8391027",     // ausente quando caps.native_id = false
      "rev": 7,                                 // inteiro local monotônico
      "first_seen": "2026-09-05T10:00:00Z",
      "last_seen": "2026-09-07T18:22:05Z"
    }
  }
}
```

`content_hash` prefixado por algoritmo (`b3:`) para que trocar de hash um dia
seja migração e não ambiguidade.

**Alternativas consideradas para o formato:**

| Opção | A favor | Custo |
|---|---|---|
| **JSON único, gravação atômica, flush com debounce** *(proposta)* | Sem dependência nova no 0.1a; legível a olho, o que importa num formato que guarda identidade; migração trivial | Reescrita O(n). Em `fixtures/large` (10k notas) o arquivo fica na casa de 2–3 MB e cada flush reescreve tudo |
| SQLite separado do índice | Escrita incremental; sem reescrita O(n) | Traz `rusqlite` para o 0.1a, que o §4 põe no 0.2. Dois bancos, dois esquemas, duas migrações — e o registro **não** é descartável como o índice, então precisaria de política diferente no mesmo motor |
| Log append-only + compactação | Barato por escrita; resistente a crash | Mais código e mais casos de borda (compactação interrompida) do que o 0.1a justifica |

**Escolha: JSON único.** O custo O(n) é aceitável no 0.1a porque nada consome
`NoteId` ainda: perder os últimos ids de um flush em debounce significa que
notas ganham ids novos ao serem vistas de novo, o que é a mesma coisa que
acontece na primeira abertura. **Isso deixa de ser verdade no 0.6**, e a decisão
tem prazo: quando o índice entrar no 0.2, reavaliar mover o registro para o
mesmo motor com tabela própria. Fica registrado como dívida, não como esquecimento.

Debounce de flush: **2 s**, mais flush imediato em fechamento de workspace, saída
do app e background no mobile.

### 1.5 `drafts/` — buffer suspenso

Escrito quando o autosave é suspenso por divergência (§12) **ou** por falha de
gravação (disco cheio, permissão, dispositivo sumiu). Some **só** quando a
gravação no destino é confirmada.

O rascunho precisa guardar os bytes **exatos** do buffer. JSON com o conteúdo
embutido obrigaria base64 (inflando 33%) ou escaping, e o §7.5 é uma política de
bytes — guardar o buffer num formato que o transforma é contraditório.

**Formato: um arquivo por rascunho, com cabeçalho emoldurado.**

```
drafts/<NoteId>.draft

<uma linha JSON com o cabeçalho>\n
<bytes crus do buffer, até o fim do arquivo>
```

```jsonc
// o cabeçalho
{
  "schema": 1,
  "note_id": "0193f2d4-...",
  "rel_path": "infra/servidor.md",       // onde estava quando suspendeu
  "reason": "diverged",                   // diverged | write_failed
  "detail": "disk_full",                  // código de CoreError quando write_failed
  "base_rev": { "size": 4213, "mtime": "...", "content_hash": "b3:9f2a..." },
  "suspended_at": "2026-09-07T19:41:00Z",
  "header_len": 412                        // bytes do cabeçalho + o \n
}
```

Um arquivo só, uma gravação atômica, bytes preservados. `header_len` deixa a
leitura ser um `seek` em vez de um scan, e a linha ser recuperável a olho se o
arquivo for lido por um humano.

**Alternativa descartada:** dois arquivos (`meta.json` + `buffer`). Precisaria
parear atomicamente dois arquivos, e um par meio gravado é exatamente o estado
que o rascunho existe para evitar.

**Retenção: nenhuma.** Rascunho não expira e não é apagado por limpeza. É a única
cópia de algo que o usuário digitou.

### 1.6 `conflicts/` — schema agora, escrita no 0.1b

A UI de resolução é 0.1b (`SCOPE §12`), então o 0.1a **não escreve aqui**. O
schema é fechado agora só para que o 0.1b não precise migrar nada.

```
conflicts/<NoteId>/<timestamp>-<side>.conflict
```

Mesmo formato emoldurado do rascunho, com `side` = `local` | `disk`,
`resolved_at`, e `resolution` = `kept_mine` | `used_disk` | `saved_as`.

**Retenção: 90 dias por default, configurável.** E a regra que vence a retenção:
**conflito não resolvido nunca é apagado por limpeza automática** (§12), qualquer
que seja a idade. A varredura só considera entradas com `resolved_at` preenchido.

### 1.7 `session.json` — estado de UI, por workspace

```jsonc
{
  "schema": 1,
  "open_tabs": [],            // 0.1c
  "active_tab": null,         // 0.1c
  "cursors": {},              // 0.1c — NoteId → { line, col, scroll }
  "sidebar_open": true
}
```

No 0.1a o arquivo existe praticamente vazio: **"persistir o último workspace" é
`workspaces.json`, não este arquivo.** As abas são 0.1c. Criar o arquivo agora,
com os campos já nomeados, evita uma migração de schema entre 0.1a e 0.1c por um
motivo que já é conhecido hoje.

---

## 2. Contrato dos commands e modelo de erros (§20.2)

### 2.1 A camada que importa não é o command

`ADR-003` já manda: a lógica vive em `crates/`, `src-tauri/` é casca fina. Então
a API de verdade é o `WorkspaceService` do `notes-core`, e os commands do Tauri
são adaptadores. Isso não é estilo — é o que faz o `notes-mcp` (0.3) funcionar
**com a janela fechada** (§15), porque ele é cliente do mesmo core e não do app.

Consequência prática para o 0.1a: **todo teste de aceite do core roda sem Tauri**
(`cargo test`, critério de aceite explícito).

### 2.2 Um command por operação, e não um `dispatch`

| Opção | A favor | Custo |
|---|---|---|
| **Um command por operação** *(proposta)* | As capabilities do Tauri são **por command** — é a granularidade que o §13 pede ("exposição mínima de commands"). Erro e argumento tipados por operação. Legível no devtools | Muitos nomes; cada um precisa ser registrado e listado na capability |
| `dispatch(Request) -> Response` tipado | Um lugar só para preocupações transversais; o mesmo envelope serviria ao `notes-mcp` | **Uma capability que concede tudo.** Permitir `dispatch` é permitir `delete`. Isso derruba o §13 inteiro, e não há como conceder metade |

**Escolha: um command por operação.** O argumento decisivo é a capability: um
`dispatch` único é um botão liga-desliga para o filesystem inteiro. As
preocupações transversais (mapeamento de erro, tracing) vão num wrapper interno
do `src-tauri`, não no formato do fio.

Superfície do 0.1a:

```
workspace_open(path)            → WorkspaceInfo
workspace_create(parent, name)  → WorkspaceInfo
workspace_restore_last()        → WorkspaceInfo | null
list_dir(rel)                   → Vec<Entry>          // lazy, um nível
note_read(rel)                  → NoteContent          // bytes + política detectada
note_write(rel, bytes, base_rev)→ WriteOutcome
note_create(rel)                → Entry                // create-new; nunca sobrescreve
dir_create(rel)                 → Entry
draft_list()                    → Vec<DraftInfo>
draft_resolve(note_id, action)  → WriteOutcome
env_report()                    → EnvReport            // diagnóstico e caps
```

`rename`, `move`, `duplicate`, `delete` e o watcher são **0.1b** e não entram —
nem "já que estou aqui" (§19).

### 2.3 Modelo de erros

O core devolve `Result<T, CoreError>`. `CoreError` é enum com **código estável**,
e o código é o contrato — a mensagem não é.

```rust
enum CoreError {
    OutsideRoot { attempted: String },   // §7.6 — nunca vaza o path absoluto real
    NotFound,
    Unavailable { reason: UnavailableReason },  // raiz sumiu, autorização revogada
    Diverged { base: BaseRev, found: BaseRev }, // §12 — não é falha, é estado
    WriteFailed { kind: WriteFailKind },        // DiskFull | PermissionDenied | Locked | Io
    ReadOnly { reason: ReadOnlyReason },        // InvalidUtf8 | MixedEol | SchemaTooNew
    AlreadyExists,
    InvalidName { rule: NameRule },             // §7.6, só para nome novo
    Busy,
}
```

**Serialização para o frontend:** `{ "code": "write_failed.disk_full", "params": { ... } }`.

**Sem texto em inglês vindo do Rust.** O §9 manda strings de UI por chave desde o
início, com `en` e `pt-BR`; se o core mandasse frase pronta, metade da UI estaria
fora do sistema de i18n no dia em que ele entra. O `code` é a chave; `params`
alimenta a interpolação.

Dois códigos merecem nota:

- **`Diverged` não é erro de falha, é estado esperado.** Ele para o autosave,
  grava o rascunho e põe a aba em "conflito". Tratar como exceção genérica é o
  caminho mais curto para "tentar de novo" sobrescrevendo o disco.
- **`WriteFailed::DiskFull` e `::PermissionDenied` são distintos** porque o
  critério de aceite do 0.1a exige erro **visível** e distinguível, e porque a
  ação do usuário difere.

### 2.4 A barra de estado é o enum, não uma string

O §9 lista **salvo · pendente · gravando · conflito · somente leitura · erro ·
indisponível**. Isso é um enum no core, derivado do estado do documento — não
texto montado na UI. Motivo: "salvo" só pode aparecer depois que o backend
confirmou (§9), e essa é uma garantia do core, não uma convenção de quem escreve
o componente.

---

## 3. Mapeamento de `Caps` (§20.6)

`Caps { atomic_replace, rename, trash, watch, native_id, preserve_mode }`.

| Backend | atomic_replace | rename | trash | watch | native_id | preserve_mode |
|---|---|---|---|---|---|---|
| ext4 / btrfs / xfs | sim (`rename(2)`) | sim | sim (XDG) | sim (inotify) | sim (dev+ino) | sim |
| APFS / HFS+ | sim | sim | sim | sim (FSEvents) | sim | sim |
| NTFS | sim, **com retry** | sim | sim (lixeira) | sim | sim (FileIndex) | parcial — ACL não é copiada |
| SMB / NFS / sshfs / FUSE de nuvem | **não garantido** | sim | não | não confiável | instável | não |
| Sandbox do app (iOS/Android) | sim | sim | não | limitado | sim | n/a |
| Bookmark iOS (0.4) | via coordenação, não POSIX | sim | não | **não** | não | n/a |
| SAF Android (0.4) | **não existe** | sim | não | **não** | não (URI muda ao mover) | n/a |

### 3.1 O que o app faz quando falta cada uma

- **Sem `atomic_replace`** → §7.1 proíbe fingir. Estratégia recuperável: grava
  tmp → fsync → **relê e confere o hash** → só então substitui, mantendo a versão
  anterior recuperável até a verificação passar. Mais lento e honesto.
- **Sem `watch`** → poll com orçamento (§7.3), mais reconciliação ao ganhar foco.
- **Sem `native_id`** → a correlação do §6.2 fica só com `ContentHash`, o que
  produz **mais** `NoteId` novo em caso ambíguo. É a direção segura já escolhida
  ("reidentificar é melhor que fundir histórico errado"), não uma degradação nova.
- **Sem `trash`** → §7.7: exclusão recuperável própria ou confirmação clara de
  permanente. Nunca `rm` silencioso.
- **Sem `preserve_mode`** → pula o passo 3 do §7.4.

### 3.2 Como as caps são descobertas — e uma armadilha

O jeito confiável de saber se um `rename` é atômico naquela raiz é testar: criar
um arquivo temporário e renomear.

**Isso é proibido.** O §2.3 diz que abrir uma pasta não a modifica, e o 0.1a tem
critério de aceite literal: *"Abrir uma pasta não cria nenhum arquivo nela."* Um
probe de escrita quebraria os dois.

**Proposta: detecção só de leitura.** `statfs`/`statvfs` no Linux e no macOS,
`GetVolumeInformationW` no Windows, para identificar o tipo de sistema de
arquivos; a tabela acima mapeia tipo → caps. Tipo desconhecido cai no perfil
**conservador** (sem `atomic_replace`, sem `trash`, sem `native_id`), que só
custa desempenho e honestidade.

Custo dessa escolha: uma raiz em FUSE que *na verdade* suporta rename atômico vai
ser tratada como se não suportasse. Preferível ao contrário — e preferível a
violar o §2.3 no primeiro segundo de uso.

---

## 4. Crates no 0.1a

```
crates/
├── notes-model/     ids, RelPath, CompareKey, BaseRev, Caps, CoreError
├── notes-fs/        trait FileSystem + caps; impl nativa desktop; política de bytes
└── notes-core/      WorkspaceService: registro, rascunhos, escrita, divergência
```

**`notes-markdown` não é criado no 0.1a.** O §5 diz que crate só existe quando o
marco que o usa começa, e no 0.1a não há consumidor: o §8.2 preserva front matter
byte a byte — o que é *não tocar*, política de bytes do `notes-fs`, não parsing —
e o §8.1 diz que o highlight do CodeMirror não é autoridade semântica. O primeiro
consumidor real é o preview, no 0.1b. **Ver §5, item 4: sua instrução listava
`notes-markdown` no 0.1a.**

Propriedade obrigatória do `notes-fs` (§19): `read(write_atomic(x)) == x` para
qualquer `bytes`, como teste de propriedade.

---

## 5. O que preciso que você confirme

1. **Bundle identifier.** Define o caminho de app data nos três SOs, entra no
   `tauri.conf.json` e **mudar depois abandona o estado de todo mundo** que já
   instalou. Sugestão: `br.com.samirhv.notes`. Alternativa sem domínio próprio:
   `io.github.samirhvbr.notes`.
2. **`workspaces.json` global.** É acréscimo meu ao layout do §6.1, e sem ele não
   há como chavear por path canônico nem persistir o último workspace. Aceita?
3. **`registry.json` já no 0.1a**, com a dívida de reavaliar o motor no 0.2
   (§1.4). A alternativa é chavear rascunho por path e aceitar que rename externo
   durante suspensão órfã o rascunho.
4. **`notes-markdown` fica para o 0.1b** (§4 acima), contra a sua instrução que o
   listava no 0.1a. Se quiser ele no 0.1a mesmo assim, ele nasce vazio.

Confirmados esses quatro, eu escrevo os ADRs das decisões irreversíveis
(identifier, formato do registro, um-command-por-operação, detecção de caps só
por leitura) e começo o 0.1a pelos crates.
