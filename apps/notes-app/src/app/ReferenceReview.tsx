import { create } from "zustand";
import { useState } from "react";
import * as ipc from "../ipc";
import { useModalSurface } from "./modal";
import { useWorkspace } from "../stores/workspace";
import { useEditor } from "../stores/editor";
import { askConfirm } from "./dialog";
import { t } from "../i18n";

type Request={plan:ipc.ReferencePlan,resolve:(selected:number[]|null)=>void};
const review=create<{current:Request|null}>(()=>({current:null}));
export function ReferenceReview(){
  const current=review(s=>s.current);
  return current?<Review key={current.plan.token} request={current}/>:null;
}
function Review({request}:{request:Request}){
  const {plan,resolve}=request;
  const [selected,setSelected]=useState(()=>plan.files.map((_,i)=>i));
  const modal=useModalSurface();
  const done=(v:number[]|null)=>{review.setState({current:null});resolve(v);};
  return <div className="overlay"><div className="dialog reference-review" {...modal} role="dialog" aria-modal="true" aria-label={t("references.title")} onKeyDown={e=>{modal.onKeyDown(e);if(e.key==="Escape")done(null);}}>
    <h2>{t("references.title")}</h2>
    <p>{plan.from} → {plan.to}</p>
    <p>{t("references.explain")}</p>
    {plan.files.map((file,i)=><div key={file.path}>
      <label><input type="checkbox" checked={selected.includes(i)} onChange={e=>setSelected(s=>e.target.checked?[...s,i]:s.filter(v=>v!==i))}/>{file.path}</label>
      {file.edits.map((edit,j)=><pre key={j}>{edit.before}{" → "}{edit.after}</pre>)}
    </div>)}
    <p>{t("references.skipped",{count:plan.skipped})}</p>
    <div className="actions"><button onClick={()=>done(null)}>{t("dialog.cancel")}</button><button className="primary" onClick={()=>done(selected)}>{t("references.apply")}</button></div>
  </div></div>;
}
export async function reviewedMove(from:ipc.RelPath,to:ipc.RelPath){
  const workspace=useWorkspace.getState().info?.id;
  const check=()=>{if(!workspace||useWorkspace.getState().info?.id!==workspace)throw new Error(t("references.expired"));};
  await useEditor.getState().save(true);
  check();
  const doc=useEditor.getState().doc;
  if(doc && doc.bufferVersion!==doc.savedVersion)throw new Error(t("references.dirty"));
  const clean=()=>{
    check();
    const active=useEditor.getState().doc;
    if(active && active.bufferVersion!==active.savedVersion)throw new Error(t("references.dirty"));
  };
  let plan:ipc.ReferencePlan;
  try {
    // Finish an existing scan, then scan again to include its intervening edits.
    for(let pass=0;pass<2;pass++){
      let status=await ipc.indexStart();
      while(status.running){
        await new Promise(r=>setTimeout(r,100));
        check();
        status=await ipc.indexStatus();
      }
      check();
    }
    plan=await ipc.referencePreview(from,to);
  } catch {
    clean();
    const proceed=await askConfirm({title:t("references.title"),body:t("references.unavailable"),confirmLabel:t("references.moveOnly")});
    if(!proceed)return null;
    clean();
    const parent=(path:string)=>path.includes("/")?path.slice(0,path.lastIndexOf("/")):"";
    const moved=parent(from)===parent(to)
      ?await ipc.entryRename(from,to.slice(to.lastIndexOf("/")+1))
      :await ipc.entryMove(from,parent(to) as ipc.RelPath);
    return {path:moved.path,updated:[],failed:[],backup:""};
  }
  clean();
  const selected=await new Promise<number[]|null>(resolve=>review.setState({current:{plan,resolve}}));
  if(selected===null)return null;
  clean();
  const result=await ipc.referenceApply(plan.token,selected);
  return result;
}
