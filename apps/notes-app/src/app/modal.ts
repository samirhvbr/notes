import { useEffect, useRef } from "react";
import type { KeyboardEvent } from "react";
const focusable='button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex="0"]';

export function useModalSurface(ready=true) {
  const ref=useRef<HTMLDivElement>(null);
  const restore=useRef<HTMLElement|null>(null);
  useEffect(()=>{
    restore.current=document.activeElement as HTMLElement;
    return ()=>{
      // An action may intentionally move focus to the editor before unmounting.
      if(document.activeElement===document.body || ref.current?.contains(document.activeElement)) restore.current?.focus();
    };
  },[]);
  useEffect(()=>{
    if(!ready)return;
    const id=requestAnimationFrame(()=>ref.current?.querySelector<HTMLElement>(focusable)?.focus());
    return ()=>cancelAnimationFrame(id);
  },[ready]);
  const onKeyDown=(e:KeyboardEvent<HTMLDivElement>)=>{
    e.stopPropagation();
    if(e.key!=="Tab")return;
    const elements=Array.from(e.currentTarget.querySelectorAll<HTMLElement>(focusable));
    const first=elements[0],last=elements[elements.length-1];
    if(e.shiftKey && document.activeElement===first){e.preventDefault();last?.focus();}
    else if(!e.shiftKey && document.activeElement===last){e.preventDefault();first?.focus();}
  };
  return {ref,onKeyDown};
}
