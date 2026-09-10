import { useEffect, useState } from "react";
import { create } from "zustand";
import * as ipc from "../ipc";
import { useWorkspace } from "../stores/workspace";
import { useEditor } from "../stores/editor";
import { useTabs } from "../stores/tabs";
import { useUi } from "../stores/ui";
import { useModalSurface } from "./modal";
import { t } from "../i18n";
function useKnowledge() {
  const [data, setData] = useState<ipc.Knowledge | null>(null);
  const [error, setError] = useState(false);
  const workspace = useWorkspace((s) => s.info?.id);
  useEffect(() => {
    setData(null);
    setError(false);
    let active = true,
      busy = false;
    const load = async () => {
      if (busy) return;
      busy = true;
      try {
        const v = await ipc.knowledgeGet();
        if (active) {
          setData(v);
          setError(false);
        }
      } catch {
        if (active) setError(true);
      } finally {
        busy = false;
      }
    };
    void load();
    const timer = setInterval(() => void load(), 3000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [workspace]);
  return { data, error };
}
function open(path: ipc.RelPath, target?: string) {
  const workspace = useWorkspace.getState().info?.id;
  void useTabs
    .getState()
    .openPath(path)
    .then(async () => {
      if (useUi.getState().panel === "graph")
        useUi.getState().togglePanel("files");
      const fragment = target?.split("#").slice(1).join("#");
      const doc = useEditor.getState().doc;
      if (!fragment || !doc || doc.path !== path) return;
      const outline = await ipc.markdownOutline(doc.text);
      if (
        useWorkspace.getState().info?.id !== workspace ||
        useEditor.getState().doc !== doc
      )
        return;
      const heading = outline.headings.find((h) => h.slug === fragment);
      if (heading) {
        const prefix = new TextDecoder().decode(
          new TextEncoder().encode(doc.text).slice(0, heading.span.start),
        );
        await useTabs.getState().openAt(path, prefix.split("\n").length, 0);
      }
    })
    .catch(useWorkspace.getState().fail);
}
export function KnowledgePanel({
  mode,
}: {
  mode: "tags" | "backlinks" | "properties";
}) {
  const { data, error } = useKnowledge();
  const doc = useEditor((s) => s.doc);
  const [tag, setTag] = useState("");
  const [metadata, setMetadata] = useState<ipc.Metadata | null>(null);
  useEffect(() => {
    let active = true;
    const timer = setTimeout(() => {
      if (doc)
        void ipc
          .metadataGet(doc.text)
          .then((v) => {
            if (active) setMetadata(v);
          })
          .catch(() => {
            if (active) setMetadata(null);
          });
      else setMetadata(null);
    }, 200);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [doc?.text]);
  const tags = [...new Set(data?.notes.flatMap((n) => n.tags) ?? [])].sort();
  const paths =
    mode === "backlinks"
      ? (data?.edges.filter((e) => e.to === doc?.path).map((e) => e.from) ?? [])
      : (data?.notes
          .filter((n) => !tag || n.tags.includes(tag))
          .map((n) => n.path) ?? []);
  return (
    <div className="side-scroll knowledge-panel">
      {error && <p role="alert">{t("index.failed")}</p>}
      {data?.partial && <p className="muted">{t("knowledge.partial")}</p>}
      {mode === "properties" ? (
        <>
          {metadata?.error && <p role="alert">{t("knowledge.invalidYaml")}</p>}
          <dl>
            {metadata?.properties.map((p) => (
              <div key={p.name}>
                <dt>{p.name}</dt>
                <dd>{p.value}</dd>
              </div>
            ))}
          </dl>
          <p>{metadata?.tags.map((v) => `#${v}`).join(" ")}</p>
          {!metadata?.properties.length && (
            <p className="muted">{t("browser.empty")}</p>
          )}
        </>
      ) : (
        <>
          {mode === "tags" && (
            <label>
              {t("browser.tags")}
              <select value={tag} onChange={(e) => setTag(e.target.value)}>
                <option value="">{t("knowledge.allTags")}</option>
                {tags.map((v) => (
                  <option key={v}>{v}</option>
                ))}
              </select>
            </label>
          )}
          {[...new Set(paths)].map((p) => (
            <button className="row" key={p} title={p} onClick={() => open(p)}>
              {p}
            </button>
          ))}
          {!paths.length && <p className="muted">{t("browser.empty")}</p>}
        </>
      )}
    </div>
  );
}
export function Graph() {
  const { data, error } = useKnowledge();
  const [query, setQuery] = useState("");
  const matching =
    data?.notes.filter((n) =>
      n.path.toLowerCase().includes(query.toLowerCase()),
    ) ?? [];
  const nodes = matching.slice(0, 80);
  const positions = new Map(
    nodes.map((n, i) => [
      n.path,
      {
        x: 320 + 240 * Math.cos((2 * Math.PI * i) / Math.max(nodes.length, 1)),
        y: 280 + 215 * Math.sin((2 * Math.PI * i) / Math.max(nodes.length, 1)),
      },
    ]),
  );
  return (
    <section className="knowledge-graph" aria-label={t("rail.graph")}>
      <h2>{t("rail.graph")}</h2>
      <input
        aria-label={t("knowledge.filter")}
        placeholder={t("knowledge.filter")}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      {error && <p role="alert">{t("index.failed")}</p>}
      {(data?.partial || matching.length > 80) && (
        <p>{t("knowledge.partial")}</p>
      )}
      <svg
        className={nodes.length > 20 ? "dense" : ""}
        viewBox="0 0 640 560"
        role="group"
        aria-label={t("knowledge.graphDescription")}
      >
        {data?.edges.map((e, i) => {
          const a = positions.get(e.from),
            b = positions.get(e.to);
          return a && b ? (
            <line key={i} x1={a.x} y1={a.y} x2={b.x} y2={b.y} />
          ) : null;
        })}
        {nodes.map((n) => {
          const p = positions.get(n.path)!;
          return (
            <g
              key={n.path}
              role="button"
              tabIndex={0}
              aria-label={n.path}
              onClick={() => open(n.path)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  open(n.path);
                }
              }}
            >
              <title>{n.path}</title>
              <rect
                className="node-hit"
                x={p.x - (nodes.length > 20 ? 14 : 70)}
                y={p.y - (nodes.length > 20 ? 14 : 30)}
                width={nodes.length > 20 ? 28 : 140}
                height={nodes.length > 20 ? 28 : 44}
              />
              <circle cx={p.x} cy={p.y} r={7} />
              <text x={p.x} y={p.y - 13} textAnchor="middle">
                {n.path.split("/").pop()?.slice(0, 24)}
              </text>
            </g>
          );
        })}
      </svg>
      <details>
        <summary>{t("knowledge.noteList")}</summary>
        {nodes.map((n) => (
          <button className="row" key={n.path} onClick={() => open(n.path)}>
            {n.path}
          </button>
        ))}
      </details>
      {!!data?.unresolved.length && (
        <details>
          <summary>
            {t("knowledge.unresolved", { count: data.unresolved.length })}
          </summary>
          {data.unresolved.map((l, i) => (
            <p key={i}>
              {l.from}: [[{l.target}]]
            </p>
          ))}
        </details>
      )}
    </section>
  );
}
const wiki = create<{
  target: string;
  paths: ipc.RelPath[];
  workspace: string | null;
}>(() => ({ target: "", paths: [], workspace: null }));
export async function openWiki(target: string) {
  const workspace = useWorkspace.getState().info?.id;
  try {
    const paths = await ipc.wikiCandidates(target);
    if (useWorkspace.getState().info?.id !== workspace) return;
    if (paths.length === 1) {
      open(paths[0], target);
      return;
    }
    wiki.setState({ target, paths, workspace: workspace ?? null });
  } catch (e) {
    useWorkspace.getState().fail(e);
  }
}
export function WikiDialog() {
  const state = wiki();
  return state.target ? <WikiChoice {...state} /> : null;
}
function WikiChoice({
  target,
  paths,
  workspace,
}: {
  target: string;
  paths: ipc.RelPath[];
  workspace: string | null;
}) {
  const modal = useModalSurface();
  const close = () => wiki.setState({ target: "", paths: [], workspace: null });
  return (
    <div className="overlay">
      <div
        className="dialog"
        {...modal}
        role="dialog"
        aria-modal="true"
        aria-label={t("knowledge.choose")}
        onKeyDown={(e) => {
          modal.onKeyDown(e);
          if (e.key === "Escape") close();
        }}
      >
        <h2>
          {t("knowledge.choose")}: {target}
        </h2>
        {!paths.length && <p>{t("knowledge.missing")}</p>}
        {paths.map((p) => (
          <button
            className="row"
            key={p}
            onClick={() => {
              close();
              if (useWorkspace.getState().info?.id === workspace)
                open(p, target);
            }}
          >
            {p}
          </button>
        ))}
        <button onClick={close}>{t("dialog.cancel")}</button>
      </div>
    </div>
  );
}
