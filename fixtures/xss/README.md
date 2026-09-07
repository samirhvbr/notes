# `fixtures/xss/`

Each file is an assertion, not a sample. The rule the renderer must satisfy
(`ARCHITECTURE.md` §10):

- no `<script>`, no event-handler attribute, and no `javascript:`,
  `data:` or `file:` URL survives into the rendered HTML;
- a remote image is blocked unless `remote_images` is on, and blocking a
  resource never stops the rest of the note rendering;
- `safe-in-code.md` must render its payloads **as text** — a renderer that
  strips them there is also wrong, because it is rewriting what the user wrote;
- `safe-links.md` must survive intact: relative note links keep
  `data-note-path`, `http(s)` links get `target=_blank rel=noopener`.

These files are exercised by `notes-markdown`, which arrives at **0.1b**. They
are committed at 0.1a so the corpus is in place before the crate that reads it.
