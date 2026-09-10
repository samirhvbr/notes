import { useEffect, useState } from "react";
import * as ipc from "../ipc";
import { t } from "../i18n";
import { useEditor } from "../stores/editor";
import { useTabs } from "../stores/tabs";
import { Tree } from "./Tree";
import { ExplorerToolbar } from "./ExplorerToolbar";

export function WorkspaceBrowser() {
  const [mode,setMode]=useState<"files"|"recent"|"outline">("files");
  const [recent,setRecent]=useState<ipc.RecentNote[]>([]);
  const [headings,setHeadings]=useState<ipc.Heading[]>([]);
  const [error,setError]=useState(false);
  const doc=useEditor(s=>s.doc);
  useEffect(()=>{
    let active=true;
    if(mode==="recent") void ipc.recentNotes().then(v=>{if(active){setRecent(v);setError(false);}}).catch(()=>{if(active)setError(true);});
    if(mode==="outline") {
      const timer=setTimeout(()=>{
        if(!doc){setHeadings([]);return;}
        void ipc.markdownOutline(doc.text).then(v=>{if(active){setHeadings(v.headings);setError(false);}}).catch(()=>{if(active)setError(true);});
      },150);
      return ()=>{active=false;clearTimeout(timer);};
    }
    return ()=>{active=false;};
  },[mode,doc?.path,doc?.text]);
  return <>
    <div className="browser-modes" role="group" aria-label={t("browser.title")}>
      {(["files","recent","outline"] as const).map(m=><button key={m} aria-pressed={mode===m} onClick={()=>setMode(m)}>{t(`browser.${m}`)}</button>)}
    </div>
    {mode==="files" ? <><ExplorerToolbar/><div className="side-scroll"><Tree/></div></> : <div className="side-scroll">
      {error && <p role="alert">{t("error.internal")}</p>}
      {mode==="recent" ? recent.map(n=><button className="row" key={n.note_id} title={n.path} onClick={()=>void useTabs.getState().openPath(n.path)}>{n.path}</button>) : headings.map((h,i)=><button className="row" key={i} onClick={()=>{
        if(!doc)return;
        const prefix=new TextDecoder().decode(new TextEncoder().encode(doc.text).slice(0,h.span.start));
        void useTabs.getState().openAt(doc.path,prefix.split("\n").length,1);
      }}>{" ".repeat(h.level-1)}{h.text}</button>)}
      {((mode==="recent" && !recent.length)||(mode==="outline" && !headings.length)) && <p className="muted">{t("browser.empty")}</p>}
    </div>}
  </>;
}
