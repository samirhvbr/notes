// `import "./styles.css"` is a real import to the bundler and nothing at all to
// the type system, and TypeScript 7 stopped letting that pass quietly: a
// side-effect import of a module with no declaration is now an error (TS2882)
// rather than an untyped shrug. Vite ships the declarations for the asset kinds
// it resolves — `*.css` among them — and this is the line that points at them.
/// <reference types="vite/client" />
