# Tura Notes identity

> **Status:** ACTIVE · Implemented in 1.0.0.

Tura Notes is the public product name; Tura is the short form. The name pays a
subtle homage to Alan Turing. A folded ribbon forms a T and evokes written
memory without depicting a person or claiming an affiliation.

The editable [app icon](assets/tura-icon.svg) and [logo](assets/tura-logo.svg)
use deep teal `#073B40`, ivory `#FAF7ED` and mint `#91C5B7`. The logo includes an
ivory background for legibility on both light and dark README themes.
The frontend copy lives in `apps/notes-app/public/tura-icon.svg`.

Regenerate platform assets from the repository root:

```sh
cd apps/notes-app
npm run tauri -- icon ../../docs/assets/tura-icon.svg --ios-color '#073B40'
```

The repository is `samirhvbr/tura-notes`. Existing checkouts can update origin
with `git remote set-url origin git@github.com:samirhvbr/tura-notes.git`.
The local checkout directory need not change.

The application identifier `br.com.samirhv.notes`, executable `notes`, Rust
crate names, package identities, environment variables and data directories
remain compatible. Branding does not migrate or reset user data. Version
1.0.0 is the owner's release designation; pending sync, mobile lifecycle and
installed-device acceptance remain tracked in `.continue/`.

## Release verification

On macOS, the 1.0.0 local DMG built successfully and passed `hdiutil verify`.
Its application metadata reports Tura Notes, version 1.0.0 and the preserved
bundle identifier. The source configuration returned to its 0.0.0 placeholder.

`tools/check.sh` passed frontend tests/build, i18n, contrast, generated types,
byte preservation, ENOSPC and the native TCP/offline restore smoke. The overall
gate remains red in unchanged Rust code: Clippy reports a needless borrow at
`crates/notes-sync-client/src/control.rs:896` on both targets, and
`starting_the_watcher_returns_immediately_and_walks_behind` fails its macOS
timing assertion. The Rust test run stops at that failure; later workspace
tests were not completed by this gate. These issues remain in the queue.
Installed UI acceptance was not performed. The local DMG is unsigned and
unnotarized and is not a published distribution artifact.
