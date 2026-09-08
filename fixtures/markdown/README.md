# `fixtures/markdown/`

The preview corpus. Each `NAME.md` is an input; the `NAME.html` beside it is the
**exact** output `notes_markdown::render_html` must produce for it — byte for
byte, no normalisation on either side.

`fixtures/xss/` is the other half and works differently: those files assert
properties (*nothing executes, nothing leaks*) rather than an exact string,
because a sanitizer is specified by what cannot survive it. See that folder's
own README.

## How to run, and how to change one

```bash
cargo test -p notes-markdown                 # compares against the committed .html
NOTES_BLESS=1 cargo test -p notes-markdown   # rewrites the .html from the renderer
```

**Blessing is not the same as accepting.** `NOTES_BLESS=1` writes what the
renderer currently produces; the diff then has to be read, line by line, against
the contract below before it is committed. A golden file that was blessed
without being read records a bug as if it were a decision — which is the one
failure mode a golden corpus has. The rule is in
[`docs/DECISIONS-0.1b.md`](../../docs/DECISIONS-0.1b.md) D-04.

## The options the corpus is rendered under

The fixed `RenderOpts` the tests use, so the files are reproducible:

| Option | Value | Why |
|---|---|---|
| `base` | the note's own directory, `""` for these files | relative links resolve from the note, not from the root |
| `raw_html` | `false` | the default, and scope §8.4: raw HTML is escaped in the MVP |
| `remote_images` | `false` | the default; a remote image is blocked and named, never fetched |
| `workspace_id` | the nil UUID | `notes-asset://` URLs carry it, and a random one would make every golden unstable |

## What each file is for

| File | The contract it pins |
|---|---|
| `headings.md` | `h1`–`h6`, ATX and setext; GitHub slugs on `id`; a repeated heading dedupes to `-1` |
| `paragraphs.md` | emphasis, strong, inline code, a hard break, `&amp;` kept as an entity, `<` escaped, thematic break |
| `lists.md` | ordered, unordered, nested, tight vs. loose |
| `tasklists.md` | GFM task items become `<input type="checkbox" disabled>`; `[X]` counts; a nested list still works |
| `table.md` | GFM tables with alignment, inline markup inside cells, a header-only table |
| `strikethrough-autolink.md` | GFM strikethrough; a bare `https://` URL becomes a link, an angle-bracketed one too, and an **email autolink does not** — see the note below |
| `code-blocks.md` | fenced with a language → `class="language-…"`; fenced without; indented; inline. **Payloads inside code render as text** |
| `blockquote.md` | quotes, nested quotes |
| `links.md` | relative note link → `data-note-path`, and `…#secao` → `data-note-anchor` beside it; anchor; `http(s)` → `target=_blank rel=noopener noreferrer`; `mailto:` **refused, text kept**; a relative link to a non-note; a link inside code reported but not rendered |
| `images.md` | relative image → `notes-asset://<workspace>/…`; remote blocked with `remote_images: false`; an empty target |
| `front-matter.md` | YAML front matter is **not rendered**, and its byte span is reported on `Document` |
| `front-matter-not-first.md` | `---` after a line of text is a thematic break, not front matter |
| `footnotes.md` | the documented extension: references and definitions |
| `html-escaped.md` | with `raw_html: false`, block and inline HTML come out as **text**, and a comment does not disappear silently |
| `unicode.md` | accents, an emoji with a ZWJ, RTL, combining marks, a zero-width space — all preserved |
| `empty.md` | an empty note renders to an empty string, not to a stray tag |
| `hard-cases.md` | nested emphasis, a lone asterisk, an accented autolink, a reference link, backslash escapes, numeric and named entities |

## Two things the corpus pins that are easy to read as bugs

**`mailto:` is not a link.** Scope §8.4: *"Links externos `http(s)` abrem no
navegador do SO por clique. Outros esquemas recusados."* The capability file
agrees — `shell:allow-open` is restricted to `http` and `https` — so rendering a
`mailto:` anchor would produce a link that does nothing when clicked. The
address stays as text. Widening the capability is the owner's act, not the
renderer's ([`docs/DECISIONS-0.1b.md`](../../docs/DECISIONS-0.1b.md) D-06).

**`www.example.com` without a scheme is not a link either.** GFM guesses
`http://` for it; guessing an insecure scheme on the user's behalf is not
something this application does quietly. `https://…` and `http://…` written out
are linkified.

## What is deliberately not here

- **Wiki links `[[…]]`, tags and attachments** — 0.3, and rendering them now
  would pin a syntax that has not been decided.
- **Rename-aware links.** 0.1 does not rewrite links when a note moves; 0.2 does,
  with the index. `Document.links` carries `in_code` so that pass can skip code.
- **Live preview and WYSIWYG** — out of the MVP by scope §9.
