// The DOM matchers `Menu.test.tsx` reads with — `toHaveTextContent`,
// `toHaveAttribute`. They are registered at runtime by `vitest.setup.ts`; this
// is the half that tells TypeScript they exist on `expect`.
/// <reference types="@testing-library/jest-dom/vitest" />
