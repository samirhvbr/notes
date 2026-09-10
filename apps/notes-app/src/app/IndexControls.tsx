import { useEffect, useState } from "react";
import * as ipc from "../ipc";
import { t } from "../i18n";

export function IndexControls({ workspace }: { workspace:string }) {
  const [status,setStatus]=useState<ipc.IndexStatus|null>(null);
  const [failed,setFailed]=useState(false);
  const [paused,setPaused]=useState(false);
  useEffect(()=>{
    let active=true, busy=false;
    const refresh=async(start:boolean)=>{
      if(busy)return;busy=true;
      try { const s=start ? await ipc.indexStart() : await ipc.indexStatus();if(active){setStatus(s);setFailed(false);if(s.stale&&!s.running&&!paused)void ipc.indexStart().catch(()=>setFailed(true));} }
      catch {if(active)setFailed(true);} finally {busy=false;}
    };
    if(!paused)void refresh(true);
    const poll=setInterval(()=>void refresh(false),500);
    const scan=setInterval(()=>{if(!paused)void refresh(true);},10000);
    return ()=>{active=false;clearInterval(poll);clearInterval(scan);};
  },[workspace,paused]);
  return <div className="index-controls" aria-live="polite">
    <span>{failed||status?.error ? t("index.failed") : status?.running ? t("index.running",{count:status.scanned}) : paused ? t("index.paused") : t("index.ready",{count:status?.scanned??0})}</span>
    {!!status?.skipped && <span>{t("index.partial",{count:status.skipped})}</span>}
    {status?.running ? <button onClick={()=>{setPaused(true);void ipc.indexCancel().catch(()=>setFailed(true));}}>{t("search.cancel")}</button> : <button onClick={()=>{setPaused(false);void ipc.indexStart(true).then(setStatus).catch(()=>setFailed(true));}}>{t("index.rebuild")}</button>}
  </div>;
}
