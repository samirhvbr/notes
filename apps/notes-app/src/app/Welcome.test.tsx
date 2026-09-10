// @vitest-environment jsdom
import {cleanup,render,screen} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {afterEach,expect,it,vi} from "vitest";
import {Welcome} from "./Welcome";
import * as ipc from "../ipc";
import {useWorkspace} from "../stores/workspace";
import {useDialog} from "./dialog";
vi.mock("@tauri-apps/plugin-dialog",()=>({open:vi.fn(async()=>"/chosen")}));
vi.mock("../ipc",async importOriginal=>({...await importOriginal<typeof import("../ipc")>(),workspaceRecent:vi.fn(async()=>[]),workspaceCreate:vi.fn(async()=>({id:"workspace"})),workspaceOpen:vi.fn()}));
afterEach(()=>{cleanup();useDialog.getState().settle(null);vi.clearAllMocks();});
it("creates a workspace through the initial screen's visible naming dialog",async()=>{
 const adopt=vi.fn(async()=>{});useWorkspace.setState({adopt,error:null});
 const user=userEvent.setup();render(<Welcome/>);
 await user.click(screen.getByRole("button",{name:/Create Workspace/}));
 const input=await screen.findByRole("textbox");await user.clear(input);await user.type(input,"my notes");
 await user.click(screen.getByRole("button",{name:"Create"}));
 expect(ipc.workspaceCreate).toHaveBeenCalledWith("/chosen","my notes");expect(adopt).toHaveBeenCalled();
});
it("cancelling creation leaves the filesystem operation uncalled",async()=>{
 const user=userEvent.setup();render(<Welcome/>);await user.click(screen.getByRole("button",{name:/Create Workspace/}));
 await screen.findByRole("textbox");await user.keyboard("{Escape}");expect(ipc.workspaceCreate).not.toHaveBeenCalled();
});
