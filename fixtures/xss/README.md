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

These files are exercised by `notes-markdown`, in
`crates/notes-markdown/tests/xss.rs`, since **0.1b**:

```bash
cargo test -p notes-markdown --test xss
```

Every `.md` here is rendered under all four combinations of `raw_html` and
`remote_images` and checked against the invariants above — so **adding a payload
to this folder is enough**; no test has to be written for it, and forgetting to
write one cannot make it pass. Each file also has a test of its own asserting it
was refused for the right reason and that the rest of the note still rendered.

The assertions are structural — tags and attributes, read back out of the
sanitized HTML — rather than substring searches. `safe-in-code.md` is why: it
must render `javascript:alert(1)` as text, so a suite that greps the output for
`javascript:` would demand the opposite of what this corpus requires.
