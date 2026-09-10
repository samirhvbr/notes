// @vitest-environment jsdom
import {cleanup,render,screen,waitFor} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {useState} from "react";
import {afterEach,expect,it} from "vitest";
import {useModalSurface} from "./modal";
function Modal({close}:{close:()=>void}){const m=useModalSurface();return <div {...m} role="dialog" onKeyDown={e=>{m.onKeyDown(e);if(e.key==="Escape")close();}}><input aria-label="Name"/><button onClick={close}>Done</button></div>;}
function Harness(){const [open,setOpen]=useState(false);return <><button onClick={()=>setOpen(true)}>Open</button>{open&&<Modal close={()=>setOpen(false)}/>}<button>Outside</button></>;}
afterEach(cleanup);
it("focuses, contains Tab and restores the opener on dismissal",async()=>{
 const user=userEvent.setup();render(<Harness/>);await user.click(screen.getByText("Open"));await waitFor(()=>expect(screen.getByRole("textbox")).toHaveFocus());
 await user.tab({shift:true});expect(screen.getByText("Done")).toHaveFocus();await user.tab();expect(screen.getByRole("textbox")).toHaveFocus();await user.keyboard("{Escape}");expect(screen.getByText("Open")).toHaveFocus();
});
