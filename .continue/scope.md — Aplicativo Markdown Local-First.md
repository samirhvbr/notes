# Scope — Aplicativo de Notas Markdown

## 1. Objetivo

Desenvolver um aplicativo de notas Markdown **local-first**, disponível inicialmente para:

- Linux
- macOS
- Windows
- iPhone / iPad
- Android

O aplicativo deve permitir ao usuário trabalhar diretamente com arquivos Markdown armazenados em uma pasta escolhida por ele.

A filosofia principal do projeto é:

> Os arquivos pertencem ao usuário, e não ao aplicativo.

O aplicativo não deverá utilizar um formato proprietário para armazenar as notas.

Os arquivos `.md` devem permanecer utilizáveis normalmente através de:

- terminal;
- VS Code;
- Git;
- rsync;
- backup;
- outros editores Markdown;
- outros aplicativos compatíveis.

---

# 2. Princípios do projeto

## 2.1 Local-first

O funcionamento básico do aplicativo não poderá depender:

- de cadastro;
- de login;
- de internet;
- de servidor;
- de serviço proprietário de nuvem.

Ao abrir o aplicativo pela primeira vez, o usuário poderá selecionar ou criar uma pasta de trabalho.

Exemplo:

```text
~/Documents/notes
```

Essa pasta será o **workspace** do usuário.

Exemplo:

```text
notes/
├── trabalho/
│   ├── projetos.md
│   └── reunioes.md
├── pessoal/
│   ├── ideias.md
│   └── viagens.md
├── servidores.md
└── tarefas.md
```

Os próprios diretórios existentes no filesystem formarão a árvore de navegação.

---

# 3. Compatibilidade dos arquivos

O formato principal será:

```text
.md
```

Markdown deve permanecer legível mesmo fora do aplicativo.

Suporte inicial:

- Markdown tradicional;
- CommonMark;
- GitHub Flavored Markdown;
- títulos;
- listas;
- listas numeradas;
- checkboxes;
- links;
- imagens;
- tabelas;
- blocos de código;
- blockquote;
- negrito;
- itálico;
- horizontal rule;
- links internos entre arquivos.

Também deverá ser suportado YAML Front Matter.

Exemplo:

```markdown
---
title: Servidores
tags:
  - infraestrutura
  - linux
created: 2026-09-07
---

# Servidores
```

O Front Matter será opcional.

O aplicativo não deverá inserir automaticamente metadados nos arquivos Markdown sem necessidade.

---

# 4. Workspace

## 4.1 Primeiro acesso

No primeiro acesso:

```text
Welcome

[ Open Folder ]
[ Create Workspace ]
```

O usuário poderá:

### Open Folder

Selecionar uma pasta já existente contendo arquivos Markdown.

### Create Workspace

Selecionar onde deseja criar uma nova pasta de notas.

Exemplo:

```text
Documents/Notes
```

---

# 5. Interface

A interface será inspirada em aplicações como Obsidian, VS Code e editores modernos de Markdown.

Não será uma cópia visual exata.

Inicialmente haverá apenas:

```text
Dark Theme
```

Não haverá sistema complexo de temas na primeira versão.

---

# 6. Layout desktop

Layout principal:

```text
┌─────────────────────────────────────────────────────────────┐
│ Toolbar                                                     │
├───────────────┬─────────────────────────────────────────────┤
│               │ tabs                                        │
│ FILES         ├─────────────────────────────────────────────┤
│               │                                             │
│ > trabalho    │                                             │
│   projetos.md │              EDITOR                         │
│   reunioes.md │                                             │
│               │                                             │
│ > pessoal     │                                             │
│   ideias.md   │                                             │
│               │                                             │
├───────────────┴─────────────────────────────────────────────┤
│ status                                                       │
└─────────────────────────────────────────────────────────────┘
```

## Sidebar

Mostrar:

- diretórios;
- arquivos Markdown;
- pesquisa;
- arquivos recentes;
- favoritos posteriormente.

Operações:

- criar nota;
- criar pasta;
- renomear;
- mover;
- excluir;
- duplicar.

Drag-and-drop deverá ser previsto para movimentação de arquivos e diretórios.

---

# 7. Editor

O editor será o componente central do aplicativo.

Tecnologia inicialmente recomendada:

```text
CodeMirror 6
```

O editor deverá ter:

- syntax highlighting Markdown;
- line numbers opcionais;
- undo/redo;
- seleção múltipla;
- atalhos de teclado;
- busca no arquivo;
- replace;
- autosave;
- detecção de modificações externas;
- suporte eficiente a arquivos grandes.

---

# 8. Modos de visualização

Inicialmente:

### Source

Markdown puro.

### Preview

Markdown renderizado.

### Split

```text
Markdown | Preview
```

Posteriormente poderá ser implementado:

### Live Preview

Visualização semelhante ao modo de edição do Obsidian, onde parte da sintaxe Markdown é renderizada diretamente durante a edição.

Live Preview **não será requisito do primeiro MVP**, pois aumenta consideravelmente a complexidade do editor.

---

# 9. Autosave

O aplicativo deverá salvar automaticamente alterações.

Não deverá existir dependência do botão:

```text
Save
```

Estratégia:

```text
alteração
   ↓
debounce
   ↓
gravação atômica
   ↓
filesystem
```

Deverá ser evitada corrupção do arquivo em caso de interrupção durante escrita.

---

# 10. Alterações externas

Uma característica obrigatória será conviver corretamente com outros programas.

Exemplo:

```text
Notes App
      ↓
arquivo.md
      ↑
VS Code / Git / script / agente IA
```

Se outro programa modificar o arquivo, o aplicativo deverá detectar a alteração.

O usuário não deverá precisar fechar e abrir novamente o workspace.

Será utilizado monitoramento do filesystem quando suportado pela plataforma.

---

# 11. Busca

O aplicativo terá dois tipos de busca.

## Busca no arquivo

```text
Ctrl+F
```

## Busca global

Pesquisar em todos os arquivos do workspace.

Pesquisar:

- nome;
- caminho;
- conteúdo.

Exemplo:

```text
firewall
```

Resultado:

```text
infra/firewall.md
Linha 24
Configuração do firewall...

clientes/projeto-x.md
Linha 73
O firewall do cliente...
```

---

# 12. Indexação

O filesystem continuará sendo a fonte de verdade.

Entretanto, poderá existir um índice local para acelerar:

- pesquisa;
- tags;
- links;
- backlinks;
- arquivos recentes;
- cache.

Tecnologia sugerida:

```text
SQLite
```

O SQLite **não armazenará a nota como fonte primária**.

Exemplo:

```text
Filesystem
    ↓
Indexer
    ↓
SQLite
```

Se o banco SQLite for apagado:

```text
reindex workspace
```

e o aplicativo deverá funcionar novamente normalmente.

---

# 13. Metadados internos

O workspace poderá possuir um diretório reservado:

```text
.notes/
```

Exemplo:

```text
notes/
├── .notes/
│   ├── workspace.json
│   ├── index.db
│   └── cache/
├── pessoal/
└── trabalho/
```

Essa pasta conterá somente dados auxiliares.

Nunca deverá conter a única cópia de uma nota.

Deverá ser possível excluir `.notes/` e reconstruí-la.

---

# 14. Links internos

Deverão ser considerados dois formatos.

Markdown padrão:

```markdown
[Servidor](infra/servidor.md)
```

E posteriormente:

```markdown
[[Servidor]]
```

O primeiro deverá fazer parte do suporte inicial.

Wiki Links poderão ser adicionados posteriormente.

---

# 15. Backlinks

Quando a indexação de links estiver implementada, o aplicativo poderá mostrar:

```text
Backlinks
```

Exemplo:

```text
servidor.md

Referenced by:

- infraestrutura.md
- projeto-datacenter.md
- checklist.md
```

Essa funcionalidade não precisa bloquear o MVP inicial.

---

# 16. Tags

Suporte futuro/próxima fase:

```markdown
#linux
#servidor
#projeto
```

e:

```yaml
tags:
  - linux
  - infraestrutura
```

O indexador deverá poder relacionar ambos.

---

# 17. Tabs

No desktop será possível abrir múltiplos arquivos em abas.

Exemplo:

```text
[ servidor.md ] [ projeto.md ] [ clientes.md ]
```

Estado das abas poderá ser restaurado ao reiniciar o aplicativo.

---

# 18. Command Palette

Atalho:

```text
Ctrl+Shift+P
```

ou:

```text
Cmd+Shift+P
```

Exemplos:

```text
New Note
New Folder
Open File
Search
Rename File
Delete File
Toggle Preview
Open Workspace
```

Isso permitirá futuramente adicionar funções sem poluir a interface.

---

# 19. Mobile

A interface mobile utilizará os mesmos conceitos, mas não tentará reproduzir o layout desktop literalmente.

Exemplo:

```text
┌───────────────────┐
│ Notes          ☰  │
├───────────────────┤
│                   │
│     Editor        │
│                   │
│                   │
└───────────────────┘
```

O sidebar será aberto através de drawer/navigation.

Prioridades no mobile:

- abrir notas;
- editar;
- criar;
- pesquisar;
- navegar entre pastas;
- sincronizar futuramente.

---

# 20. Abstração de filesystem

O frontend não deverá conhecer detalhes específicos do filesystem.

Arquitetura:

```text
UI
 │
 ▼
WorkspaceService
 │
 ▼
FileSystemAdapter
 │
 ├── DesktopFileSystem
 ├── IOSFileSystem
 ├── AndroidFileSystem
 └── RemoteFileSystem        futuro
```

Operações mínimas:

```text
list()
read()
write()
create()
rename()
move()
delete()
stat()
watch()
```

Isso é importante para permitir diferentes comportamentos entre desktop, iOS e Android.

---

# 21. Arquitetura sugerida

Stack inicial:

```text
Tauri 2
React
TypeScript
Rust
CodeMirror 6
SQLite
```

Estrutura aproximada:

```text
src/
├── components/
├── editor/
├── explorer/
├── search/
├── workspace/
├── services/
├── hooks/
└── stores/

src-tauri/
├── filesystem/
├── workspace/
├── indexer/
├── search/
└── commands/
```

Separação:

```text
React
    UI

TypeScript
    estado e lógica da interface

Rust
    filesystem
    indexação
    busca
    operações locais
    integração nativa

SQLite
    índice/cache/metadados
```

---

# 22. Estado da aplicação

O estado da UI deverá ser separado dos arquivos.

Exemplos:

```text
workspace atual
arquivo ativo
abas abertas
sidebar aberta
posição do cursor
scroll
arquivos recentes
```

Pode ser utilizado um state manager leve.

Exemplo:

```text
Zustand
```

---

# 23. Segurança local

Por padrão:

```text
nenhuma API de rede aberta
```

O aplicativo desktop não deverá abrir portas TCP automaticamente.

Qualquer integração externa deverá ser explicitamente habilitada.

---

# 24. Self-hosted Sync

A sincronização **não fará parte do MVP inicial**.

Entretanto, a arquitetura deverá prever sua implementação.

Posteriormente será criado um servidor:

```text
Notes Server
```

Hospedável pelo próprio usuário.

Exemplo:

```text
docker compose up -d
```

Arquitetura futura:

```text
Desktop ──┐
          │
iPhone ───┼──── HTTPS ─── Notes Server
          │
Android ──┘
```

O servidor armazenará os arquivos remotamente e permitirá sincronização entre dispositivos.

---

# 25. Princípio da sincronização

A sincronização não poderá depender apenas de:

```text
modified_at
```

Cada arquivo deverá futuramente possuir informações suficientes para identificação de:

```text
file_id
path
revision
content_hash
modified_at
device_id
```

O protocolo também deverá prever:

```text
deleted files / tombstones
conflicts
renames
offline modifications
version history
```

Mesmo que esses recursos não sejam implementados imediatamente, o modelo deverá evitar decisões que impossibilitem sua implementação posterior.

---

# 26. Conflitos

Exemplo:

```text
Desktop modifica projeto.md
          ↓

iPhone modifica projeto.md
          ↓

ambos estavam offline
```

O sistema nunca deverá simplesmente sobrescrever silenciosamente uma das versões.

Possíveis tratamentos futuros:

```text
projeto.md
projeto (conflict iphone).md
```

ou interface específica para resolução de conflitos.

---

# 27. Servidor self-hosted

Futuramente o servidor deverá oferecer:

```text
Authentication
Workspaces
Files
Sync
Versions
API
Agents
```

O servidor deverá funcionar em container.

Exemplo:

```text
notes-server
```

com volume:

```text
/data
```

Os dados deverão continuar administráveis pelo proprietário do servidor.

---

# 28. API pública

Quando o servidor self-hosted existir, ele poderá expor uma API REST.

Exemplo:

```text
GET    /api/v1/workspaces
GET    /api/v1/files
GET    /api/v1/files/{id}
POST   /api/v1/files
PUT    /api/v1/files/{id}
DELETE /api/v1/files/{id}
GET    /api/v1/search
```

Também poderão existir operações semânticas:

```text
append
prepend
rename
move
```

---

# 29. API para agentes de IA

Um dos objetivos explícitos do projeto será permitir integração com agentes de IA.

Exemplo:

```text
AI Agent
    │
    ▼
Notes API
    │
    ▼
Workspace
```

O agente poderá, de acordo com suas permissões:

```text
listar arquivos
pesquisar arquivos
ler arquivos
criar arquivos
editar arquivos
renomear arquivos
mover arquivos
```

Exemplo:

```text
GET /api/v1/files/projetos/erp.md
```

ou:

```text
PUT /api/v1/files/projetos/erp.md
```

---

# 30. Permissões para agentes

Tokens deverão possuir escopos.

Exemplo:

```text
notes:read
notes:write
notes:create
notes:delete
search:read
```

Será possível criar um agente apenas de leitura.

Exemplo:

```text
Token:
read-only
```

Esse token não poderá alterar nenhuma nota.

---

# 31. MCP

Além da API REST, deverá ser considerada uma interface:

```text
Model Context Protocol — MCP
```

Isso permitiria integrar diretamente o workspace com agentes e aplicações de IA compatíveis com MCP.

Exemplo futuro:

```text
tools:

notes_list
notes_search
notes_read
notes_create
notes_update
notes_move
```

A API REST continuará sendo a interface genérica.

O MCP atuará como uma camada específica para agentes.

---

# 32. Histórico de versões

O servidor remoto poderá futuramente manter histórico.

Exemplo:

```text
projeto.md

v1
v2
v3
v4
```

Permitindo:

```text
View History
Compare
Restore
```

Esse recurso não deverá alterar o formato do arquivo Markdown local.

---

# 33. Git

O aplicativo não dependerá de Git.

Entretanto, como os arquivos são Markdown normais, um workspace poderá ser um repositório Git.

Exemplo:

```text
notes/
├── .git/
├── trabalho/
└── pessoal/
```

O aplicativo não deverá impedir esse uso.

Integração nativa com Git poderá ser considerada posteriormente.

---

# 34. Atalhos

Desktop deverá priorizar navegação por teclado.

Exemplos:

```text
Ctrl+N          New Note
Ctrl+P          Quick Open
Ctrl+Shift+F    Search Workspace
Ctrl+F          Search File
Ctrl+W          Close Tab
Ctrl+S          Force Save
Ctrl+Shift+P    Command Palette
```

No macOS deverão existir equivalentes utilizando `Cmd`.

---

# 35. Quick Open

Deverá existir uma pesquisa rápida por arquivo.

Exemplo:

```text
Ctrl+P

> serv
```

Resultado:

```text
servidor.md
servidores/dns.md
servidores/email.md
```

Essa função deverá funcionar independentemente da pesquisa textual global.

---

# 36. Configurações iniciais

A primeira versão terá poucas configurações.

Exemplo:

```text
Editor
 ├─ font size
 ├─ line numbers
 ├─ word wrap
 └─ tab size

Files
 ├─ autosave
 └─ show hidden files

Markdown
 └─ default preview mode
```

Não haverá sistema de plugins no MVP.

---

# 37. Performance

O aplicativo deverá continuar utilizável com workspaces contendo:

```text
10 arquivos
100 arquivos
1.000 arquivos
10.000+ arquivos
```

O carregamento inicial não deverá exigir abrir o conteúdo de todos os arquivos simultaneamente.

Indexação deverá ocorrer incrementalmente.

---

# 38. MVP 0.1 — Desktop

Primeiro objetivo funcional:

```text
Linux
macOS
Windows
```

Recursos:

- iniciar aplicação;
- selecionar workspace;
- listar diretórios;
- listar `.md`;
- criar arquivo;
- abrir arquivo;
- editar Markdown;
- autosave;
- renomear;
- excluir;
- criar diretório;
- mover arquivo;
- syntax highlighting;
- preview Markdown;
- busca no arquivo;
- quick open;
- persistir workspace selecionado.

Quando isso estiver funcionando, teremos um editor Markdown local realmente utilizável.

---

# 39. MVP 0.2 — Indexação

Adicionar:

- SQLite;
- indexação incremental;
- pesquisa global;
- arquivos recentes;
- detecção de alterações externas;
- restauração das abas;
- command palette.

---

# 40. MVP 0.3 — Markdown avançado

Adicionar:

- YAML Front Matter;
- tags;
- links internos;
- backlinks;
- imagens;
- attachments;
- tabelas;
- melhorias no preview.

---

# 41. MVP 0.4 — Mobile

Adicionar:

```text
iOS
Android
```

Recursos prioritários:

- selecionar workspace;
- navegar;
- abrir;
- editar;
- criar;
- pesquisar;
- autosave.

A interface deverá ser adaptada para mobile em vez de simplesmente reduzir o desktop.

---

# 42. Versão 0.5 — Self-hosting

Criar:

```text
Notes Server
```

Com:

- autenticação;
- workspace remoto;
- armazenamento;
- API REST;
- tokens;
- controle de permissões;
- Docker;
- versionamento básico.

---

# 43. Versão 0.6 — Sync

Implementar:

- dispositivos;
- revisões;
- hashes;
- tombstones;
- sync incremental;
- offline mode;
- conflitos;
- resolução de conflitos;
- histórico.

---

# 44. Versão 0.7 — IA

Adicionar:

```text
REST API
MCP Server
```

Operações:

- list;
- search;
- read;
- create;
- update;
- append;
- move.

Sempre utilizando autenticação e scopes.

---

# 45. Recursos fora do escopo inicial

Não implementar no MVP:

- colaboração simultânea;
- editor WYSIWYG completo;
- canvas;
- graph view;
- sistema de plugins;
- múltiplos temas;
- publicação web;
- chat de IA embutido;
- integração Git nativa;
- sync;
- histórico de versões;
- contas de usuário;
- servidor cloud oficial.

Esses recursos poderão ser avaliados posteriormente.

---

# 46. Regra arquitetural principal

A arquitetura deverá obedecer:

```text
Markdown files = source of truth

SQLite = index/cache

Notes Server = sync/remote storage

REST API = integrations

MCP = AI agents
```

Nunca:

```text
SQLite
   ↓
única cópia da nota
```

E nunca:

```text
formato proprietário
   ↓
exportar Markdown depois
```

O Markdown deverá existir no filesystem desde o primeiro momento.

---

# 47. Visão final

A evolução pretendida é:

```text
                     ┌──────────────┐
                     │  AI Agents   │
                     └──────┬───────┘
                            │
                       REST / MCP
                            │
                            ▼
Linux ─────┐          ┌──────────────┐
macOS ─────┤          │ Notes Server │
Windows ───┼── Sync ──│ Self Hosted  │
iPhone ────┤          └──────────────┘
Android ───┘
     │
     ▼
 Local Workspace
     │
     ▼
Markdown Files
```

O aplicativo deve começar simples:

```text
pasta + markdown + editor
```

e evoluir para:

```text
local-first
     +
multi-device sync
     +
self-hosting
     +
API
     +
AI/MCP
```
