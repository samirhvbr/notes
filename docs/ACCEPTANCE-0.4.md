# Acceptance — milestone 0.4

> **Status:** `ACTIVE` tracking document · Foundation integrated in 0.17.0.
> The usable mobile milestone is not complete.

PR #2 makes the trash crate a desktop-only dependency and reports
`Caps::LOCAL.trash = false` on iOS/Android. Mobile deletion therefore reports
`DeleteOutcome::Permanent`; the existing desktop trash and guarded atomic
write behavior are retained. ADRs 040 and 042 record the integration choices.

The regression gate includes an iOS simulator cross-check of notes-model,
notes-fs, notes-markdown, notes-index and notes-core, alongside desktop tests.
This is compilation evidence, not a running mobile app or Android proof.

The following implementation and owner checks remain pending:

- Tauri mobile entry point and generated Apple/Android projects.
- Full-screen editor, drawer navigation and Markdown keyboard toolbar.
- Application-container workspace flows and background flush.
- iOS security-scoped bookmarks and Android SAF with persisted authorization.
- Revocation, moved documents, provider offline and reauthorization handling.
- Budgeted polling for providers without watch support.
- Real virtual-keyboard input: accents, dead keys, IME, selection, paste and undo.
- Physical-device and installed-release owner acceptance and repeat.

The owner explicitly requested proceeding to milestone 0.5 after merging this
foundation. That scheduling choice does not check off these mobile criteria.
