// Every user-visible string is a key from the start (scope §9). `en` and
// `pt-BR` are embedded; the default follows the operating system's locale.
//
// Error text is keyed by `CoreError.code`, which is why the core sends a code
// and never a sentence.
import en from "./en.json";
import ptBR from "./pt-BR.json";

type Catalogue = Record<string, string>;
const catalogues: Record<string, Catalogue> = { en, "pt-BR": ptBR };

function pick(): string {
  const want = navigator.language ?? "en";
  if (catalogues[want]) return want;
  const base = want.split("-")[0];
  return Object.keys(catalogues).find((k) => k.split("-")[0] === base) ?? "en";
}

let locale = pick();

export function setLocale(l: string) {
  if (catalogues[l]) locale = l;
}

/** Missing keys render as the key itself — visible, never blank. */
export function t(key: string, params: Record<string, string | number> = {}): string {
  const raw = catalogues[locale]?.[key] ?? catalogues.en[key] ?? key;
  return raw.replace(/\{(\w+)\}/g, (_, p) => String(params[p] ?? `{${p}}`));
}
