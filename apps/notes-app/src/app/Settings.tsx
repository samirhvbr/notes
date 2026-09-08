import { useEffect, useState } from "react";
import { t } from "../i18n";
import * as ipc from "../ipc";
import type { Settings as CoreSettings } from "../ipc";
import { setLocale } from "../i18n";

/**
 * The minimum settings of milestone 0.1c: font size, line numbers, word wrap,
 * tab size — plus the locale, because `en`/`pt-BR` is in the same milestone and
 * a language nobody can choose is not shipped.
 *
 * Every change is written through `settings_set` immediately. There is no Save
 * button here for the same reason there is none for a note: a settings panel
 * that can be closed with unsaved changes is a way to lose them.
 */
export function SettingsPanel({
  onClose,
  onChanged,
}: {
  onClose: () => void;
  onChanged: (s: CoreSettings) => void;
}) {
  const [settings, setSettings] = useState<CoreSettings | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ipc.settingsGet().then(setSettings).catch((e) => setError(String(e)));
  }, []);

  if (!settings) return null;

  const write = async (next: CoreSettings) => {
    setSettings(next);
    onChanged(next);
    try {
      await ipc.settingsSet(next);
      setError(null);
    } catch (e) {
      setError(t(`error.${ipc.asCoreError(e).code}`));
    }
  };

  const editor = settings.editor;

  return (
    <div
      className="overlay"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="dialog settings"
        role="dialog"
        aria-modal="true"
        aria-label={t("settings.title")}
        onKeyDown={(e) => {
          if (e.key === "Escape") onClose();
        }}
      >
        <h2>{t("settings.title")}</h2>

        <label htmlFor="set-font">{t("settings.fontSize")}</label>
        <input
          id="set-font"
          type="number"
          min={9}
          max={32}
          value={editor.font_size}
          onChange={(e) =>
            void write({
              ...settings,
              editor: { ...editor, font_size: clamp(Number(e.target.value), 9, 32) },
            })
          }
        />

        <label htmlFor="set-tab">{t("settings.tabSize")}</label>
        <input
          id="set-tab"
          type="number"
          min={1}
          max={8}
          value={editor.tab_size}
          onChange={(e) =>
            void write({
              ...settings,
              editor: { ...editor, tab_size: clamp(Number(e.target.value), 1, 8) },
            })
          }
        />

        <label className="check">
          <input
            type="checkbox"
            checked={editor.line_numbers}
            onChange={(e) =>
              void write({ ...settings, editor: { ...editor, line_numbers: e.target.checked } })
            }
          />
          {t("settings.lineNumbers")}
        </label>

        <label className="check">
          <input
            type="checkbox"
            checked={editor.word_wrap}
            onChange={(e) =>
              void write({ ...settings, editor: { ...editor, word_wrap: e.target.checked } })
            }
          />
          {t("settings.wordWrap")}
        </label>

        <label htmlFor="set-locale">{t("settings.language")}</label>
        <select
          id="set-locale"
          value={settings.ui.locale}
          onChange={(e) => {
            setLocale(e.target.value);
            void write({ ...settings, ui: { ...settings.ui, locale: e.target.value } });
          }}
        >
          <option value="auto">{t("settings.language.auto")}</option>
          <option value="en">English</option>
          <option value="pt-BR">Português (Brasil)</option>
        </select>

        {error && <p className="bad">{error}</p>}

        <div className="actions">
          <button type="button" className="primary" onClick={onClose}>
            {t("settings.done")}
          </button>
        </div>
      </div>
    </div>
  );
}

function clamp(n: number, lo: number, hi: number): number {
  if (!Number.isFinite(n)) return lo;
  return Math.min(Math.max(Math.round(n), lo), hi);
}
