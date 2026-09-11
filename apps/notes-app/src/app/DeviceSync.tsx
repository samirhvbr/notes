import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import * as ipc from "../ipc";
import { t } from "../i18n";
import { useWorkspace } from "../stores/workspace";
import { errorText } from "./StatusBar";

/** Network hints are advisory. Unknown power/connection conservatively pauses
 * unless the owner explicitly permits it. Never infer AC power from no API. */
export async function conditions(): Promise<ipc.SyncConditions> {
  const host = navigator as Navigator & {
    connection?: { saveData?: boolean; effectiveType?: string; type?: string };
    getBattery?: () => Promise<{ charging: boolean }>;
  };
  const network = host.connection;
  let charging: boolean | null = null;
  if (host.getBattery) {
    try { charging = await Promise.race([host.getBattery().then(b => b.charging), new Promise<null>(resolve => setTimeout(() => resolve(null), 1000))]); } catch { /* unknown */ }
  }
  // Wi-Fi does not prove an unmetered connection; only an explicit browser
  // save-data signal proves restriction. Unknown remains unknown.
  return { online: navigator.onLine === true, metered: network?.saveData === true || network?.type === "cellular" ? true : null, charging };
}
const initial: ipc.SyncPairRequest = {state_dir:"",source:"",origin:"",workspace:"",scope:null,token_file:"",allow_private:false,mode:"reconcile"};
const defaults = {enabled:false,interval_seconds:300,allow_metered:false,allow_battery:false};

export function DeviceSync() {
  const workspace = useWorkspace(s => s.info);
  const [snapshot,setSnapshot]=useState<ipc.DeviceSnapshot|null>(null);
  const [request,setRequest]=useState(initial);
  const [settings,setSettings]=useState<ipc.SyncConnection|null>(null);
  const [preview,setPreview]=useState<ipc.SyncPairPreview|null>(null);
  const [busy,setBusy]=useState(false);
  const [message,setMessage]=useState("");
  const [chosen,setChosen]=useState<{local:string;remote:string}|null>(null);
  const [path,setPath]=useState("");
  const [result,setResult]=useState("");
  const [deleting,setDeleting]=useState(false);
  async function refresh() {
    const next=await ipc.deviceStatus();setSnapshot(next);
    setSettings(old=>old ?? next.connection);
  }
  useEffect(()=>{
    let alive=true, running=false;
    const poll=async()=>{
      if(running)return;running=true;
      try {
        await ipc.deviceConditions(await conditions());
        const next=await ipc.deviceStatus();
        if(alive){setSnapshot(next);setSettings(old=>old??next.connection);}
      } catch { /* Standalone browser and transient busy reads retain the view. */ }
      finally {running=false;}
    };
    void poll();const timer=setInterval(()=>void poll(),15000);
    window.addEventListener("online",poll);window.addEventListener("offline",poll);
    return()=>{alive=false;clearInterval(timer);window.removeEventListener("online",poll);window.removeEventListener("offline",poll);};
  },[]);
  async function task(work:()=>Promise<void>) {
    if(busy)return;setBusy(true);setMessage("");
    try {await work();await refresh();}
    catch(e){setMessage(errorText(ipc.asCoreError(e)));try{await refresh();}catch{/* keep error */}}
    finally{setBusy(false);}
  }
  async function pick(key:"state_dir"|"source"|"token_file") {
    const value=await open({directory:key!=="token_file",multiple:false,title:t(`device.${key}`)});
    if(typeof value==="string")setRequest(r=>({...r,[key]:value}));
  }
  async function attach() {
    const c={...defaults,state_dir:request.state_dir,token_file:request.token_file};
    await ipc.deviceConfigure(c);setSettings(c);setPreview(null);
  }
  async function pair() {
    await ipc.devicePair(request);await attach();
    if(request.mode!=="upload")setPreview(await ipc.devicePreview());
  }
  const blocked=busy||!!workspace;
  const conflicts=preview?.rows.some(r=>r.action==="conflict")||!!preview?.attachment_conflicts.length;
  const phase=snapshot?.phase??"disabled";
  return <details className="device-sync">
    <summary>{t("device.title")} · <span role="status">{t(`device.phase.${phase}`)}</span>{snapshot?.connection&&` · ${t("device.counts",{pending:snapshot.pending,received:snapshot.unapplied})}`}</summary>
    <div className="device-body">
      <p>{t("device.explain")}</p>
      {snapshot?.connection&&<button onClick={()=>void ipc.devicePause().then(async()=>{const next=await ipc.deviceStatus();setSnapshot(next);setSettings(next.connection);}).catch(e=>setMessage(errorText(ipc.asCoreError(e))))}>{t("device.pause")}</button>}
      {snapshot?.reason&&<p role="status">{["offline","network_limited_or_unknown","power_limited_or_unknown","saved_receiver_changes"].includes(snapshot.reason)?t(`device.reason.${snapshot.reason}`):snapshot.reason}</p>}
      <p role="status" aria-live="polite">{message}</p>
      <fieldset disabled={busy}><legend>{t("device.connection")}</legend>
        <div className="device-fields">
          {(["source","state_dir","token_file"] as const).map(key=><label key={key}>{t(`device.${key}`)}<span><input value={request[key]} onChange={e=>setRequest({...request,[key]:e.target.value})}/><button type="button" onClick={()=>void pick(key)} aria-label={t("device.chooseField",{field:t(`device.${key}`)})}>{t("device.choose")}</button></span></label>)}
          <label>{t("device.server")}<input type="url" placeholder="https://notes.example.com" value={request.origin} onChange={e=>setRequest({...request,origin:e.target.value})}/></label>
          <label>{t("device.remoteWorkspace")}<input value={request.workspace} onChange={e=>setRequest({...request,workspace:e.target.value})}/></label>
          <label>{t("device.scope")}<input value={request.scope??""} onChange={e=>setRequest({...request,scope:e.target.value||null})}/></label>
          <label>{t("device.mode")}<select value={request.mode} onChange={e=>setRequest({...request,mode:e.target.value})}>{["upload","download","reconcile"].map(m=><option key={m} value={m}>{t(`device.mode.${m}`)}</option>)}</select></label>
        </div>
        <label><input type="checkbox" checked={request.allow_private} onChange={e=>setRequest({...request,allow_private:e.target.checked})}/>{t("device.private")}</label>
        <div className="device-actions"><button disabled={blocked||!request.source||!request.state_dir||!request.token_file||!request.origin||!request.workspace} onClick={()=>void task(pair)}>{t("device.pair")}</button><button disabled={!request.state_dir||!request.token_file} onClick={()=>void task(attach)}>{t("device.attach")}</button></div>
        {!!workspace&&<p>{t("device.closeFirst")}</p>}
      </fieldset>
      {settings&&<fieldset disabled={busy}><legend>{t("device.schedule")}</legend>
        <p>{settings.state_dir}</p>
        <label><input type="checkbox" checked={settings.enabled} onChange={e=>setSettings({...settings,enabled:e.target.checked})}/>{t("device.enabled")}</label>
        <label>{t("device.interval")}<input type="number" min={120} max={3600} value={settings.interval_seconds} onChange={e=>setSettings({...settings,interval_seconds:Number(e.target.value)})}/></label>
        <label><input type="checkbox" checked={settings.allow_metered} onChange={e=>setSettings({...settings,allow_metered:e.target.checked})}/>{t("device.metered")}</label>
        <label><input type="checkbox" checked={settings.allow_battery} onChange={e=>setSettings({...settings,allow_battery:e.target.checked})}/>{t("device.battery")}</label>
        <div className="device-actions"><button onClick={()=>void task(()=>ipc.deviceConfigure(settings))}>{t("device.save")}</button><button onClick={()=>void task(async()=>{await ipc.deviceConditions(await conditions());await ipc.deviceRun();})}>{t("device.run")}</button><button disabled={blocked||!snapshot?.receive} onClick={()=>void task(async()=>setPreview(await ipc.devicePreview()))}>{t("device.preview")}</button><button disabled={blocked||!snapshot?.receive||!!preview} onClick={()=>void task(()=>ipc.deviceApply())}>{t("device.apply")}</button></div>
      </fieldset>}
      {preview&&<section><h3>{t("device.review")}</h3><ul>{preview.rows.map((row,i)=><li key={i}>{t(`device.action.${row.action}`)} — {row.path}</li>)}{preview.attachment_conflicts.map(p=><li key={p}>{t("device.action.conflict")} — {p}</li>)}</ul><button disabled={blocked||conflicts} onClick={()=>void task(async()=>{await ipc.deviceConfirm(preview.confirmation);setPreview(null);})}>{t("device.confirm")}</button></section>}
      {!!snapshot?.conflicts.length&&<section><h3>{t("device.conflicts")}</h3><button disabled={blocked} onClick={()=>void task(()=>ipc.deviceRecapture())}>{t("device.recapture")}</button><ul>{snapshot.conflicts.map((c,i)=><li key={i}>{snapshot.history.find(r=>r.note===c.note)?.path??c.note} {c.collision?t("device.collision"):<button disabled={busy} onClick={()=>{setChosen(c);setPath(snapshot.history.find(r=>r.id===c.local)?.path??"");setResult("");setDeleting(false);}}>{t("device.resolve")}</button>}</li>)}</ul></section>}
      {chosen&&<fieldset disabled={busy}><legend>{t("device.resolve")}</legend>
        <label>{t("device.resultPath")}<input value={path} onChange={e=>setPath(e.target.value)}/></label>
        <label><input type="checkbox" checked={deleting} onChange={e=>setDeleting(e.target.checked)}/>{t("device.deleteChoice")}</label>
        {!deleting&&<label>{t("device.resultFile")}<input value={result} onChange={e=>setResult(e.target.value)}/><button onClick={()=>void open({multiple:false,directory:false}).then(p=>{if(typeof p==="string")setResult(p);})}>{t("device.choose")}</button></label>}
        <button disabled={blocked||!path||(!deleting&&!result)} onClick={()=>void task(async()=>{await ipc.deviceResolve(chosen.local,chosen.remote,path,deleting?null:result);setChosen(null);setMessage(t("device.resolved"));})}>{t("device.stageChoice")}</button>
      </fieldset>}
      {!!snapshot?.history.length&&<section><h3>{t("device.history")}</h3><ul className="device-history">{snapshot.history.map((r,i)=><li key={`${r.id}-${i}`}><span title={r.id}>{r.path} · {t(r.deleted?"device.deleted":r.branch?"device.branch":r.pending?"device.queued":"device.received")}</span><div className="device-actions">
        {!r.deleted&&<button disabled={busy} onClick={()=>void task(async()=>setMessage(await ipc.deviceExport(r.id)))}>{t("device.export")}</button>}
        {snapshot.receive&&!r.pending&&<button disabled={blocked} onClick={()=>void task(()=>ipc.deviceCapture(r.note))}>{t("device.capture")}</button>}
        {snapshot.receive&&!r.pending&&r.resolution&&<button disabled={blocked} onClick={()=>void task(()=>ipc.deviceApply(r.id))}>{t("device.applyChoice")}</button>}
        {r.attachments.map(a=><button key={a} disabled={busy} onClick={()=>void task(async()=>setMessage(await ipc.deviceExport(r.id,a)))}>{t("device.exportAsset",{path:a})}</button>)}
      </div></li>)}</ul></section>}
    </div>
  </details>;
}
