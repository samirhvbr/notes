import { useState } from "react";
import { t } from "../i18n";

export function PdfImport({ name: initialName, text, onCancel, onSave }: {
  name: string;
  text: string;
  onCancel: () => void;
  onSave: (name: string, text: string) => Promise<void>;
}) {
  const [name, setName] = useState(initialName);
  const [value, setValue] = useState(text);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const save = async () => {
    if (!name.trim()) return setError(t("dialog.nameRequired"));
    setSaving(true);
    setError(null);
    try { await onSave(name, value); } catch { setError(t("error.internal")); }
    finally { setSaving(false); }
  };

  return <div className="modal-backdrop" role="presentation">
    <section className="dialog pdf-import" role="dialog" aria-modal="true" aria-labelledby="pdf-import-title">
      <h2 id="pdf-import-title">{t("pdf.title")}</h2>
      <p>{t("pdf.body")}</p>
      <label htmlFor="pdf-import-name">{t("dialog.name")}</label>
      <input id="pdf-import-name" value={name} onChange={(e) => setName(e.target.value)} disabled={saving} />
      <label htmlFor="pdf-import-text">{t("pdf.text")}</label>
      <textarea id="pdf-import-text" value={value} onChange={(e) => setValue(e.target.value)} disabled={saving} />
      {error && <p className="bad" role="alert">{error}</p>}
      <div className="actions">
        <button type="button" onClick={onCancel} disabled={saving}>{t("dialog.cancel")}</button>
        <button type="button" className="primary" onClick={() => void save()} disabled={saving}>{t("pdf.save")}</button>
      </div>
    </section>
  </div>;
}
