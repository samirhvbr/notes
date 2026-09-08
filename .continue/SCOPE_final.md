# SCOPE — Notes (nome provisório)

Aplicativo de notas Markdown local-first. Documento **v2.0** — 2026-09-07.

Este documento fecha o *quê* e as decisões que, se ficarem abertas, viram rewrite. O *como* (tipos, schema, contrato dos commands) vai para o `ARCHITECTURE.md`; mudança de decisão fechada aqui exige ADR.

**O que mudou da v1**

- Dados fora dos `.md` agora têm três categorias (conteúdo / operacional / derivado). Só o derivado é descartável.
- Nenhum `id` no front matter, nem ao ligar sync. Identidade vive no registro do app.
- Hash deixa de ser identidade; vira correlação com regras de ambiguidade.
- Guarda de concorrência entra junto com o autosave, no 0.1a — não depois.
- Marco 0.0 (spike) para provar mobile e Linux/Wayland antes de consolidar.
- Arch Linux entra como plataforma explícita, com distribuição via AUR.
- MCP local roda sem a janela do app; coordenação de escrita entre processos definida.
- Crates vazios removidos; libs viram "referência", trocar exige ADR.
- Rascunho recuperável e cópias de conflito viram estado persistente com retenção própria.

---

## 0. Em uma frase

Uma pasta de `.md` escolhida pelo usuário, um editor bom, e nada que quebre quando ele abre a mesma pasta no Vim, no Git ou num agente de IA.

## 1. Tese

- Os arquivos pertencem ao usuário. O app é um cliente.
- O disco é a fonte de verdade das notas. O app guarda estado próprio fora da pasta e sabe a diferença entre o que é cache e o que não é.
- Local-first de verdade: zero conta, zero rede, zero servidor para o uso básico.
- Preparado para sync, API e agentes desde o **modelo de dados** — não desde o código.

Ordem de prioridade quando duas coisas conflitam: **preservar conteúdo → editar com conforto → encontrar → integrar agentes → sincronizar**.

## 2. Princípios não negociáveis

1. Nenhum formato proprietário. `.md` legível em qualquer editor, sempre.
2. O app nunca escreve num `.md` o que o usuário não digitou. Sem título, id, tag, data, reflow ou newline "corrigida". Transformação pedida pelo usuário (inserir link, colar imagem) é edição, não metadado.
3. Abrir uma pasta não a modifica. Nenhum arquivo ou diretório é criado na pasta sem ação explícita do usuário.
4. Alteração pendente ou divergência detectada nunca é descartada nem sobrescrita em silêncio — nem entre dispositivos (futuro), nem entre o app e o VS Code (0.1a).
5. O frontend não toca em filesystem. Toda IO passa pelo core em Rust, que valida path e autorização por conta própria — capabilities do Tauri não substituem isso.
6. O core funciona sem UI e sem janela aberta. UI, MCP e API são clientes do mesmo core, com as mesmas regras.
7. Índice e cache são descartáveis. Registro de identidade, rascunhos e cópias de conflito **não são**.

## 3. Plataformas, distribuição e CI

| Plataforma | Papel | Distribuição |
|---|---|---|
| Debian/Ubuntu (GNOME, X11 e Wayland) | dev primário + release | `.deb` + AppImage |
| Arch Linux (GNOME Wayland) | dev + release | AUR (`notes-bin` a partir da release; `notes-git` opcional) |
| macOS (Apple Silicon; Intel se custar zero) | release | `.dmg` assinado e notarizado |
| Windows 10/11 x64 | release | MSI/NSIS assinado (sem assinatura, SmartScreen bloqueia) |
| iPhone (iOS 17+) | release 0.4 | TestFlight → App Store |
| Android 11+ | release 0.4 | APK na release + Play depois |
| iPad / tablets | depois do 0.4 | mesma build, layout revisto |

Notas de plataforma que já são requisito:

- **Arch é rolling.** WebKitGTK atualiza sem aviso; CI roda um job em container Arch contra `webkit2gtk-4.1` atual, não só Debian stable.
- **Wayland + NVIDIA + WebKitGTK** tem histórico de janela preta e flicker. O app detecta o cenário e aplica o workaround conhecido (`WEBKIT_DISABLE_DMABUF_RENDERER`) no startup, com opção para desligar. É item de aceite do 0.0.
- Build iOS exige macOS/Xcode; build Android roda no Linux. Contas Apple Developer e Google Play são pré-requisito do 0.4, não bloqueiam antes.
- "Compila" não é "suportado". Plataforma anunciada = CI verde + instalação testada + fluxo de arquivos validado.

## 4. Stack

| Camada | Referência | Nota |
|---|---|---|
| Shell | Tauri 2 | Decidido (já é a stack de outro app em produção do time). O 0.0 valida mobile e Wayland, não valida Tauri |
| UI | React + TypeScript + Zustand | |
| Editor | CodeMirror 6 | `@codemirror/lang-markdown` + GFM. Highlighting próprio; não é a autoridade semântica |
| Core | Rust | Toda a lógica de workspace |
| Markdown | `pulldown-cmark` + `ammonia` | Parser semântico único e sanitização |
| Índice | SQLite (`rusqlite` bundled) + FTS5 | A partir do 0.2 |
| Watcher | `notify` + debouncer | Desktop; mobile faz poll |
| Hash | `blake3` | Conteúdo |
| Lixeira | crate `trash` | Desktop |

As bibliotecas são **escolha de referência**: o agente usa estas sem discutir; trocar exige ADR com motivo. Isso evita bikeshedding sem congelar o projeto.

Sem `tauri-plugin-fs` exposto ao frontend. Capabilities do frontend: janela, dialog (seletor de pasta), clipboard, shell-open. Nada de FS.

## 5. Arquitetura em uma tela

```
apps/notes-app/          Tauri + React (cliente)
crates/
  notes-model/           ids, paths, erros, tipos
  notes-fs/              trait FileSystem + capabilities; impl desktop; watcher; mobile no 0.4
  notes-markdown/        parse, AST, sanitize, links, front matter
  notes-core/            WorkspaceService: workspace, registro, escrita, concorrência
  notes-index/           sqlite, fts, links, tags          (criado no 0.2)
  notes-mcp/             binário stdio                      (criado no 0.3)
server/                                                     (criado no 0.5)
fixtures/                workspaces de teste
```

Crate só existe quando o marco que o usa começa. Sem "só tipos até o 0.6".

```
React ──(tauri commands)──▶ notes-core ──▶ notes-fs ──▶ disco
notes-mcp ──(stdio)───────▶ notes-core      │
                                │           └── watcher / poll
                                ├──▶ notes-markdown
                                └──▶ notes-index (0.2+)
```

## 6. Modelo de dados

### 6.1 Três categorias de dado

| Categoria | Exemplos | Onde | Pode apagar? |
|---|---|---|---|
| **Conteúdo** | `.md`, anexos | `<workspace>/` | é do usuário; entra no backup dele |
| **Operacional** | registro de identidade, rascunhos recuperáveis, cópias de conflito, estado de sync (0.6), abas/cursor | app data, por workspace | não como cache; tem retenção e migração próprias |
| **Derivado** | `index.db`, cache de preview, thumbnails | app data, por workspace | sim; reconstrói do conteúdo |
| **Configuração portável** (opcional) | regras de ignore, pasta de anexos | `<workspace>/.notes/` | só existe se o usuário ligar; apagar perde preferências, não notas |
| **Credenciais** | tokens de servidor/agentes | keychain do SO | nunca no workspace, no Git ou no índice |

App data: `<app>/workspaces/<WorkspaceId>/{registry.json, drafts/, conflicts/, index.db, session.json}`.

`WorkspaceId` é atribuído pelo app ao registrar a pasta, chaveado pelo path canônico da raiz. Se a pasta for movida, o app oferece "reconectar" pelo registro; se não reconhecer, é um workspace novo. Não exige `.notes/` para nada disso.

`index.db` nunca fica dentro da pasta por default: banco SQLite ativo copiado por Dropbox/iCloud/Nextcloud no meio de uma transação vira cópia inconsistente.

### 6.2 Identidade

**Path não é identidade. Hash também não.**

```
WorkspaceId   uuid, app data
NoteId        uuid, gerado na primeira vez que a nota é vista; persiste em registry.json
RelPath       relativo à raiz, como está no disco (nunca reescrito)
CompareKey    NFC + case-fold conforme o FS da raiz; só para comparar e detectar colisão
Size, Mtime   sinais
ContentHash   blake3; recalculado quando size/mtime mudam ou em reindex
NativeId      inode (Unix) / FileIndex (Windows) quando o FS fornece; sinal forte, não prova
BaseRev       versão do arquivo que o buffer aberto viu (size + mtime + hash)
Rev           inteiro local monotônico por nota
```

Regras:

- Rename/move feito **pelo app** preserva o `NoteId`. Sempre.
- Alteração **externa**: o core correlaciona por `NativeId`, depois por `ContentHash`, e só reconecta o `NoteId` quando a correspondência é única (um path sumiu, um apareceu, mesmo hash, sem outro candidato). Duas notas vazias, uma nota duplicada, ambiguidade de qualquer tipo → `NoteId` novo. Reidentificar é melhor que fundir histórico errado.
- **Nada disso vai para o `.md`.** Ligar sync (0.6) não insere `id:` em lugar nenhum; a identidade entre dispositivos é do registro local + servidor. Se um dia existir "identidade portável no front matter", é uma feature separada, opt-in, com o usuário ciente de que os arquivos mudam.
- Copiar a pasta para outro lugar cria outro workspace com ids novos. Reconectar ao servidor é fluxo explícito (0.6), nunca automático.

## 7. Filesystem

### 7.1 Trait com capabilities

```rust
trait FileSystem {
    fn caps(&self) -> Caps;   // atomic_replace, rename, trash, watch, native_id, preserve_mode
    fn list(&self, dir: &RelPath) -> Result<Vec<Entry>>;
    fn read(&self, p: &RelPath) -> Result<Bytes>;
    fn write_atomic(&self, p: &RelPath, data: &[u8], expect: Option<&BaseRev>) -> Result<Stat>;
    fn create_dir(&self, p: &RelPath) -> Result<()>;
    fn rename(&self, from: &RelPath, to: &RelPath) -> Result<()>;
    fn delete(&self, p: &RelPath) -> Result<DeleteOutcome>;   // Trashed | Permanent
    fn stat(&self, p: &RelPath) -> Result<Stat>;
    fn watch(&self) -> Result<EventStream, Unsupported>;
}
```

`move` é `rename` no mesmo backend; entre volumes/provedores só no 0.4+ e como copy+delete validado. `duplicate` é `read` + `write_atomic` sem `expect`, com `create-new` (nunca sobrescreve destino).

O adapter declara o que garante. Backend que não tem replace atômico não finge: restringe escrita ou usa estratégia recuperável validada (write + verify + rename do backup).

### 7.2 Três tipos de workspace

| Tipo | Desktop | Mobile |
|---|---|---|
| Path nativo | caso principal | não existe |
| Árvore autorizada (iOS security-scoped bookmark / Android SAF) | raro | quando o usuário escolhe pasta externa |
| Sandbox do app | fallback | default do "Create Workspace" |

No iOS, `Documents` do app fica visível no app Arquivos (`UIFileSharingEnabled` + `LSSupportsOpeningDocumentsInPlace`). No Android, o equivalente é armazenamento específico do app — some ao desinstalar, e o app avisa isso na criação. Import/export/share dos `.md` existe no 0.4, antes de qualquer servidor.

Perder acesso à raiz (unidade removida, autorização revogada, documento movido no SAF) produz estado **indisponível**. Workspace inacessível não é workspace vazio — nada de tombstone, nada de "sumiram 400 notas".

### 7.3 `watch()` não é universal

- Desktop: `notify` com debounce ~200ms; eventos normalizados para `Created | Modified | Removed | Renamed`.
- Mobile / árvore autorizada: `Unsupported` → poll por `stat` em foreground, scan completo ao voltar do background. Sempre com orçamento (N arquivos por tick) para não travar UI.
- Reconciliação também roda ao ganhar foco, ao trocar de aba e por comando manual. Evento de watcher é sinal; a decisão é sempre `stat`/hash.
- Escrita atômica (`tmp + rename`) chega em vários SOs como `Removed + Created`. O core reconhece self-write (path + hash esperado) e não perde aba/cursor/id. A marca de self-write expira: uma alteração externa diferente logo em seguida não pode ser engolida.

### 7.4 Escrita atômica

1. `tmp` no mesmo diretório (`.nome.md.tmp-<rand>`).
2. write + fsync.
3. Copiar modo/permissões do original quando `caps.preserve_mode`.
4. Replace: `rename` no Unix; no Windows, replace com retry curto (AV e indexadores seguram handle).
5. Inode muda; hard links quebram. Aceito e documentado.

Isso resolve **arquivo truncado**. Não resolve **quem escreveu por último** — isso é §12.

### 7.5 Política de bytes

- UTF-8, com ou sem BOM. Bytes inválidos → abre somente leitura, com aviso. Sem conversão silenciosa.
- BOM, `LF`/`CRLF` e newline final são detectados no load e reaplicados no save. **EOL misto** → somente leitura até o usuário autorizar conversão (CodeMirror normaliza quebras internamente; fingir que preserva seria mentira).
- Save grava o buffer byte a byte. Sem trim, sem reflow, sem reordenar YAML, sem serializar AST.
- Hash do buffer == hash do último salvo → sem write, sem mtime novo.

Teste obrigatório: abrir → salvar sem editar → `git status` limpo, para todo arquivo dos fixtures.

### 7.6 Paths, nomes e escape

- Raiz canonicalizada na abertura; **toda** operação revalida o path resolvido contra a raiz no momento do uso — não só na abertura.
- Path de API/command: relativo, sem `..` após resolução fora da raiz, sem absoluto. Link Markdown `../outra/nota.md` é legítimo e resolve a partir da nota de origem; só é recusado se a resolução sair da raiz.
- Symlinks e junctions **não são atravessados** no MVP: aparecem na árvore como tal, não abrem. Opt-in depois.
- Nomes existentes no disco nunca são reescritos. Nome incompatível com outro SO (`:`, `?`, `CON`, trailing dot) é sinalizado, não renomeado.
- Nome **novo** (criar/renomear) segue regra portátil: sem `\ / : * ? " < > |`, sem reservados do Windows, sem trailing dot/space, sem colisão por `CompareKey`.
- Extensões de nota: `.md`, `.markdown`. Ignorados por default: `.notes/`, `.git/`, `.obsidian/`, diretórios iniciados com `.`, temporários do próprio app. Lista explícita, configurável; não adota `.gitignore` do usuário como política de visibilidade.

### 7.7 Delete

- Desktop: lixeira do SO quando `caps.trash`; o resultado (`Trashed` vs `Permanent`) é mostrado ao usuário. Sem lixeira → exclusão recuperável própria (`conflicts/`-like em app data, com retenção) ou confirmação clara de permanente. Nunca fallback silencioso para `rm`.
- Nota com edição pendente: resolver ou preservar o buffer antes de excluir.
- Diretório não vazio: confirmação com escopo. Falha parcial reporta o que foi de fato afetado.
- Excluir/renomear nota não apaga anexos: podem ser compartilhados. Limpeza de órfãos é função futura, explícita e revisável.

## 8. Markdown

### 8.1 Um parser

`notes-markdown` é a única autoridade semântica: preview, outline, links, tags, front matter e índice saem dele. O CodeMirror faz highlighting com gramática própria — apresentação, não semântica.

Perfil 0.1: CommonMark + GFM (tabelas, tasklists, strikethrough, autolinks). Footnotes documentadas como extensão. O perfil do app é o que está documentado, não "tudo que o GitHub renderiza".

### 8.2 Front matter

- YAML entre `---` na primeira linha. Opcional. YAML inválido não impede editar a nota como texto.
- Preservado byte a byte desde o 0.1a. Interpretado (título, tags, propriedades) só pelo índice, no 0.3.
- O app nunca o reescreve.

### 8.3 Links

- 0.1: `[texto](caminho/relativo.md#titulo-opcional)`, resolvido a partir da pasta da nota. Clique abre.
- 0.1: rename/move **não** reescreve links. Limitação informada na UI.
- 0.2: com índice, rename/move mostra a lista de referências afetadas — nos dois sentidos: links que apontam para a nota e links relativos que ela contém — e aplica após confirmação. Não mexe em blocos de código. Alteração multi-arquivo tem verificação de concorrência por arquivo e relatório de falha parcial; não é transação atômica de filesystem.
- 0.3: `[[wiki links]]`. Ambíguo → pergunta; nunca abre um homônimo arbitrário.

### 8.4 HTML e sanitização

- Nota é conteúdo não confiável, mesmo local. Raw HTML é escapado no MVP; renderizado só opt-in por workspace e sempre via `ammonia` com allowlist.
- Preview não executa JS, não abre frames, não submete forms, não invoca commands nativos.
- Imagens: só relativas ao workspace, formatos suportados. Remotas bloqueadas por default; opt-in. Bloquear recurso não impede ler o resto da nota.
- Links externos `http(s)` abrem no navegador do SO por clique. Outros esquemas recusados. Link não dispara shell.
- CSP estrita na WebView; exposição mínima de commands.

## 9. Editor e visualização

- Modos: Source, Preview, Split. Live Preview e WYSIWYG fora do MVP.
- Preview renderizado pelo core com debounce; arquivo grande não trava digitação.
- Barra de estado distingue: **salvo · pendente · gravando · conflito · somente leitura · erro · indisponível**. Estado "salvo" só após o backend confirmar.
- Autosave: debounce ~750ms (configuração inicial) + flush em blur, troca de aba, fechar aba, fechar app, background no mobile. `Ctrl+S` força.
- Gravações serializadas por documento. Cada gravação carrega `(buffer_version, BaseRev)`. Gravação antiga que termina depois de novas teclas não marca o buffer atual como salvo.
- Abas com cursor/scroll restaurados (0.1c). Uma janela, um workspace ativo; trocar de workspace resolve pendências antes.
- Atalhos (Cmd no macOS): `Ctrl+N` nova nota · `Ctrl+P` quick open · `Ctrl+F` busca no arquivo · `Ctrl+Shift+F` busca global · `Ctrl+W` fecha aba · `Ctrl+Shift+P` palette · `Ctrl+E` alterna preview.
- Dark theme só. Inter na UI, JetBrains Mono no editor. Foco visível, navegação por teclado, estado nunca só por cor.
- Strings da UI por chave desde o início; `en` e `pt-BR` embarcados, default pelo locale do SO.

## 10. Busca: grep primeiro, índice depois

- 0.1c: busca global por varredura no core (`ignore` crate + regex), resultados em streaming com linha e contexto, cancelável. Pesquisa conteúdo **salvo** — a UI diz isso quando há pendências.
- Quick open: fuzzy sobre paths em memória. Não depende de índice.
- 0.2: FTS5 assume busca por palavras; grep continua para literal e regex. As três semânticas (literal / palavras / regex) têm nome na UI e não trocam por baixo.
- Falha ou reconstrução do índice não impede abrir e editar.

## 11. Índice (0.2)

- SQLite em app data. Tabelas: `notes` (id, path, size, mtime, hash, rev), `links`, `tags`, `fts` (unicode61, `remove_diacritics`).
- Incremental: `(size, mtime)` mudou → rehash; hash mudou → reparse. Alteração externa invalida derivados. Reindex sob comando, com indicador de progresso; resultado parcial é marcado como parcial.
- Reindexar não altera notas nem `NoteId`.

## 12. Concorrência e conflito (0.1a)

Existe antes de qualquer servidor: autosave pendente + VS Code ou agente gravando o mesmo arquivo.

| Situação | Comportamento |
|---|---|
| Disco mudou, buffer limpo | recarrega, preserva cursor quando possível |
| Disco mudou, buffer sujo | **suspende autosave** do documento; preserva as duas versões; aba em "conflito" |
| Disco já tem exatamente o conteúdo do buffer | convergência; sem conflito |
| Arquivo removido externamente com edição local | preserva buffer, oferece recuperar; não recria o path sozinho |
| Raiz/arquivo inacessível | estado indisponível; não infere exclusão |

Mecânica:

- Antes de gravar, o core compara `BaseRev` com o `stat` atual; size+mtime iguais → grava; diferentes → hash; hash diferente → conflito. Size/mtime sozinhos nunca autorizam sobrescrever.
- Buffer suspenso por conflito ou por falha de gravação (disco cheio, permissão, lock, dispositivo sumiu) é persistido em `drafts/` em app data. Rascunho só some quando a gravação no destino é confirmada.
- Resolução (UI no 0.1b): comparar · manter o meu · usar o do disco · salvar como `nome (local).md`. A versão não escolhida vai para `conflicts/` com retenção configurável; conflito não resolvido nunca é apagado por limpeza automática.
- **Entre processos do próprio produto** (app + `notes-mcp`): lock advisory por workspace em app data (`write.lock`, flock/LockFileEx) envolvendo a seção crítica *stat → compare → replace*. Usar o mesmo crate não compartilha lock; o lock compartilha.
- **Programas de terceiros** não participam do lock. O compromisso é: nunca sobrescrever divergência detectada, nunca perder buffer conhecido. Não é exclusão mútua universal, e o scope não promete isso.

Isso é o ensaio do protocolo de sync.

## 13. Segurança

- Nenhuma porta TCP aberta pelo desktop. Integrações são explicitamente habilitadas.
- Frontend sem capability de FS; core valida path e autorização em toda operação (§7.6).
- Preview sanitizado, CSP estrita (§8.4).
- Sem telemetria. Uso local não envia nada a ninguém.
- Autorizar um agente a ler notas significa que o cliente de IA processa esse conteúdo conforme a configuração dele. O app não chama isso de "privacidade local".

## 14. Mobile (0.4)

- Mesma UI React, layout próprio: editor em tela cheia, drawer para árvore/busca/quick open. Barra de atalhos Markdown acima do teclado.
- "Create Workspace" → sandbox do app. "Open Folder" → plugin nativo: Swift com security-scoped bookmark; Kotlin com SAF + `takePersistableUriPermission`. Antes de escrever do zero, avaliar o que o `tauri-plugin-fs` já cobre no iOS.
- Permissão revogada, documento movido, provedor offline → indisponível, com reautorização. SAF persistido não garante acesso depois que o documento é movido.
- `ScopedFileSystem` sem `watch`, com poll orçado.
- Flush ao entrar em background. Sem promessa de sync ou execução contínua em background.
- Aceite inclui: acentos e dead keys (`ã`, `ç`), composição IME, seleção, colar, undo com teclado virtual — cobertos já no 0.0, confirmados no 0.4.

## 15. Agentes e MCP local (0.3)

- `notes-mcp`: binário stdio, iniciado pelo cliente MCP, sem porta TCP. **Funciona sem a janela do app aberta** — é cliente do `notes-core`, não do app.
- Tools: `notes_list`, `notes_search`, `notes_read`, `notes_create`, `notes_update`, `notes_append`, `notes_move`. `notes_delete` existe, desligado por default, permissão separada.
- Autorização por workspace e, opcionalmente, por subpasta. `read / create / update / move / delete / search` são permissões distintas. Busca não vaza trecho de nota fora da autorização.
- Toda escrita informa a `BaseRev` que o agente leu; base diferente → recusa identificável. `append` idempotente por `(NoteId, BaseRev)`: retry após timeout não duplica texto.
- Modo revisão: restringir o agente a `proposals/` e aplicar depois por ação humana.
- Conteúdo de nota nunca é interpretado como instrução para ampliar permissão ou executar comando.
- Essas permissões governam esta integração. Não restringem um agente que já tem acesso amplo à máquina por outro caminho.

## 16. Servidor, API e sync (0.5–0.7)

Nada disso é MVP. O modelo (§6) e a política de concorrência (§12) existem para que isso seja adição, não rewrite.

- **0.5 `notes-server`**: um proprietário, seus workspaces, dispositivos e agentes. Sem orgs, equipes ou permissões empresariais. Container com volume `/data`, HTTPS obrigatório para exposição remota, backup/restore documentados, notas continuam `.md` em diretório. API REST versionada com OpenAPI; "pública" = documentada e autenticada, não anônima. Uma convenção de endereçamento (path *ou* id), não duas. Credencial por integração, revogável, escopo mínimo; escrita condicional por `If-Match`/versão esperada; limites de tamanho, paginação, rate limit, autoria; logs sem token e sem conteúdo integral.
- **0.6 sync**: protocolo próprio de revisões — identidade, ancestralidade, renames, tombstones, confirmações. `modified_at` e relógio local não decidem vencedor. `notes-sync` fica **ao lado** do FS, não como `RemoteFileSystem.write()`. Primeiro pareamento distingue *enviar pasta existente / baixar remoto / reconciliar dois lados com conteúdo*; nunca substituição em massa sem identificar conflitos. Estados: desativado · offline · pendente · sincronizando · atualizado · conflito · erro. Sync nativo + outro sincronizador bidirecional na mesma pasta não é configuração suportada. WebDAV pode existir como *acesso* opcional; não é o mecanismo de sync.
- **0.7 MCP remoto**: o mesmo `notes-mcp` falando com o servidor. Uma lógica de permissão, não duas.
- Sem E2EE na primeira versão remota: o servidor precisa ler para servir API e busca. Sync, lixeira e histórico não são backup; a documentação diz isso.
- Migrações de estado persistente têm versionamento, cópia prévia e recuperação. Nunca reescrevem notas.

## 17. Roadmap com critérios de aceite

Números são marcos, não datas. Ambiente de referência para tempos: máquina de dev com NVMe, `fixtures/large` (10k arquivos, ~200 MB), cold start.

### 0.0 — Spike (timebox: 2 semanas, nada vira produto)

Escopo: shell Tauri em Debian (X11 e Wayland), Arch (Wayland + NVIDIA), macOS, Windows · CodeMirror 6 em iPhone e Android · escolher pasta, persistir autorização, gravar, fechar e reabrir (iOS bookmark, Android SAF).

Aceite:
- [ ] Janela renderiza corretamente em Arch/Wayland/NVIDIA com o workaround aplicado automaticamente
- [ ] Digitar `ação`, `ç` via dead key e emoji no editor mobile sem duplicação/perda; selecionar, colar, undo funcionam
- [ ] Pasta autorizada continua acessível após matar e reabrir o app nas duas plataformas
- [ ] `docs/SPIKE-0.0.md` registra limitações encontradas e o que do `tauri-plugin-fs` serve no iOS

### 0.1a — Edição local segura

Escopo: abrir/criar workspace · árvore lazy · abrir, editar, criar nota e pasta · syntax highlight · autosave atômico com `BaseRev` · rascunho recuperável · barra de estado · persistir último workspace · dark theme.

Aceite:
- [ ] `fixtures/basic` e `fixtures/large` listam a árvore em <1s sem ler conteúdo
- [ ] Matar o processo durante 1000 saves em loop nunca deixa arquivo truncado ou vazio
- [ ] Abrir cada arquivo dos fixtures e salvar sem editar → `git status` limpo (CRLF, BOM, sem newline final, NFD, EOL misto abre read-only)
- [ ] Buffer sujo + `echo x >> nota.md` externo → autosave suspende, nada sobrescrito, rascunho em app data — teste automatizado no core
- [ ] Disco cheio / permissão negada → erro visível, buffer recuperável ao reabrir
- [ ] Nenhum command aceita path resolvido fora da raiz (`..`, absoluto, symlink) — teste no core
- [ ] Abrir uma pasta não cria nenhum arquivo nela
- [ ] `cargo test` nos crates passa sem Tauri

### 0.1b — Organização e leitura

Escopo: rename, move, duplicate, delete (lixeira) · watcher + reconciliação em foco · UI de conflito · preview e split · busca/substituição no arquivo.

Aceite:
- [ ] Editar no VS Code com o app aberto atualiza a aba em <1s sem perder cursor
- [ ] Rename via app não reseta aba/cursor/id; rename externo não ambíguo reconecta; ambíguo gera id novo — testes no core
- [ ] Criar/duplicar nunca sobrescreve destino existente; move com colisão pede resolução
- [ ] `fixtures/xss/` no preview não executa script nem carrega recurso externo
- [ ] Delete reporta `Trashed` ou `Permanent`; nunca apaga sem dizer qual

### 0.1c — Navegação

Escopo: quick open · busca global por grep · abas com restauração · command palette · configurações mínimas (font, line numbers, wrap, tab size) · `en`/`pt-BR`.

Aceite:
- [ ] Busca global em `fixtures/large` entrega o primeiro resultado em <500ms e é cancelável
- [ ] Reabrir o app restaura workspace, abas, aba ativa e cursor

### 0.1d — Interface

Escopo: a casca da aplicação. Referência de layout: **Obsidian dark**. Regra:
paleta, espaçamento e estrutura são livres; arquivo de tema, CSS ou asset do
Obsidian **não se copia — reconstrói**. Ícones: `lucide-react` (ISC).

Estrutura, da esquerda para a direita:

- **Faixa vertical de ícones**: arquivos, busca, graph (desabilitado, com
  tooltip "0.3"); configurações embaixo.
- **Barra lateral**: explorador com toolbar (nova nota, nova pasta, ordenar,
  recolher tudo); no rodapé, o **seletor de workspace** — nome atual + chevron
  abrindo menu com *Open folder…*, *Create workspace…*, recentes e *Close
  workspace* (volta para a Welcome; respeita `DirtyBuffers`).
- **Área central**: barra de abas (as abas do 0.1c, fechar, `+`, split);
  cabeçalho da nota com voltar/avançar, título centralizado, alternar
  Source/Preview, menu; editor em coluna centralizada de largura máxima ~700 px,
  tipografia com H1 grande. Split abre painel à direita.
- **Barra de estado**, canto inferior direito: os sete estados do §9, palavras,
  caracteres. Backlinks é 0.3 — **não pôr contador falso**.
- Fundo, superfícies e destaque num tom só de escuro com um acento; contraste AA
  no texto.

Aceite:
- [ ] Trocar de workspace sem passar pela Welcome, pelo seletor no rodapé da
      barra lateral, respeitando buffers sujos
- [ ] Contraste AA no texto, verificado automaticamente
- [ ] Menus navegáveis por teclado
- [ ] Os 25 fluxos do 0.1b/0.1c repassados na interface nova: os passos mudam de
      lugar, o comportamento não

**MVP desktop = 0.1a + 0.1b + 0.1c + 0.1d.** Release para Debian, Arch, macOS e Windows.

### 0.2 — Índice e referências
SQLite em app data, incremental, FTS5, recentes, outline, rename com revisão de links nos dois sentidos. Aceite: reindexar não altera notas nem ids; semânticas de busca não mudam por baixo.

### 0.3 — Conhecimento e agentes locais
Front matter interpretado, tags (`#tag` e YAML, nunca de dentro de código), wiki links, backlinks, **graph view** (depois dos backlinks, que são o dado de que ele vive), colar imagem → `attachments/` na raiz com nome não colidente, `notes-mcp`. Aceite: app e MCP gravando o mesmo arquivo não se sobrescrevem (teste com dois processos); MCP funciona com o app fechado.

### 0.4 — Mobile utilizável (§14)
### 0.5 — Servidor e REST
### 0.6 — Sync
### 0.7 — MCP remoto

## 18. Fora de escopo até segunda ordem

Colaboração em tempo real / CRDT · Live Preview / WYSIWYG · canvas · plugins e marketplace · temas além do dark · publicação web · chat de IA embutido · integração Git nativa · cloud oficial · multi-janela · orgs/equipes · app web · OCR · busca semântica por embeddings · E2EE na primeira versão remota.

*Graph view saiu desta lista em 08/09/2026 e passou a ser 0.3, depois dos backlinks.*

Recurso fora de escopo não entra como "melhoria incidental" de um agente. Entra por revisão deste documento.

## 19. Regras para agentes

- Cada crate tem testes próprios e roda sem Tauri. `fixtures/`: `basic`, `large` (gerado por script), `edge-cases` (CRLF, BOM, NFD, EOL misto, vazio, duplicado, 5 MB, nomes com espaço/acento/colisão por caixa), `xss/`.
- Propriedade obrigatória em `notes-fs`: `read(write_atomic(x)) == x` para qualquer `bytes`.
- CI: Debian, Arch (container), macOS, Windows. Sem verde nos quatro, marco desktop não fecha.
- Nada de decisão arquitetural "no caminho". Detalhe interno e reversível o agente resolve; qualquer coisa que toque integridade, formato dos arquivos, rede, permissões, dependência central ou escopo, ele para e submete.
- Ordem estrita: 0.1a inteiro, com aceite, antes de qualquer item do 0.1b. Sem "já que estou aqui".
- Toda entrega diz: o que foi feito, quais critérios passaram, limitações conhecidas, documentos alterados. Compilar não conclui marco.

## 20. Decisões a fechar no ARCHITECTURE.md

1. Schema de `registry.json`, `drafts/`, `conflicts/`, `session.json`; retenção e migração.
2. Contrato dos Tauri commands (um por operação vs. `dispatch` tipado) e modelo de erros do core.
3. IR entre `notes-markdown` e preview: HTML sanitizado vs AST + render JS. Tendência: HTML; AST só para outline/links.
4. Lock entre processos: formato, timeout, comportamento com lock órfão.
5. `.notes/` portável: o que entra quando o usuário liga, e como se comporta se versionado no Git.
6. Mapeamento de `Caps` por backend (ext4, btrfs, APFS, NTFS, SAF, bookmark iOS) e o que cada um perde.
7. Distribuição: pipeline de assinatura macOS/Windows, PKGBUILD do AUR, AppImage.
