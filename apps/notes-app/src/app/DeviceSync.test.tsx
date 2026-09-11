// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { DeviceSync, conditions } from "./DeviceSync";
import * as ipc from "../ipc";
import { useWorkspace } from "../stores/workspace";
vi.mock("../ipc", async original=>({...await original<typeof import("../ipc")>(),deviceStatus:vi.fn(),deviceConditions:vi.fn(async()=>{}),deviceConfigure:vi.fn(async()=>{}),devicePair:vi.fn(async()=>{}),devicePreview:vi.fn(),deviceConfirm:vi.fn(async()=>{}),deviceRun:vi.fn(async()=>{}),deviceApply:vi.fn(async()=>{})}));
vi.mock("@tauri-apps/plugin-dialog",()=>({open:vi.fn()}));
const empty:ipc.DeviceSnapshot={receive:false,connection:null,phase:"disabled",reason:null,pending:0,unapplied:0,history:[],conflicts:[]};
const old=useWorkspace.getState();
beforeEach(()=>{useWorkspace.setState({info:null});vi.mocked(ipc.deviceStatus).mockResolvedValue(empty);});
afterEach(()=>{cleanup();useWorkspace.setState(old);vi.clearAllMocks();});
function show(){render(<DeviceSync/>);fireEvent.click(screen.getByText(/Device sync/));}
it("treats unavailable network and power information as unknown",async()=>{
  expect(await conditions()).toEqual({online:true,metered:null,charging:null});
});
it("reconnects with scheduling disabled and preserves conservative limits",async()=>{
  show();
  fireEvent.change(screen.getByRole("textbox",{name:/Private sync queue folder/}),{target:{value:"/private/queue"}});
  fireEvent.change(screen.getByRole("textbox",{name:/Credential file/}),{target:{value:"/private/token"}});
  fireEvent.click(screen.getByRole("button",{name:"Reconnect existing queue"}));
  await waitFor(()=>expect(ipc.deviceConfigure).toHaveBeenCalledWith({state_dir:"/private/queue",token_file:"/private/token",enabled:false,interval_seconds:300,allow_metered:false,allow_battery:false,capture_saved:false}));
  expect(ipc.deviceRun).not.toHaveBeenCalled();
});
it("requires review and refuses to confirm divergent pairing rows",async()=>{
  vi.mocked(ipc.devicePreview).mockResolvedValue({confirmation:"bound-snapshot",rows:[{action:"conflict",path:"a.md"}],attachment_conflicts:[]});
  show();
  for(const [label,value] of [[/Local notes folder/,"/notes"],[/Private sync queue folder/,"/queue"],[/Credential file/,"/token"],[/Server address/,"https://notes.example"],[/^Server workspace$/,"home"]] as const){fireEvent.change(screen.getByRole("textbox",{name:label}),{target:{value}});}
  fireEvent.click(screen.getByRole("button",{name:"Create pairing and review"}));
  const button=await screen.findByRole("button",{name:"Confirm this pairing"});
  expect(button).toBeDisabled();expect(ipc.deviceConfirm).not.toHaveBeenCalled();
  expect(ipc.devicePair).toHaveBeenCalledOnce();
});
it("does not apply received files while an editor workspace is open",async()=>{
  useWorkspace.setState({info:{id:"workspace",root:"/notes"} as ipc.WorkspaceInfo});
  vi.mocked(ipc.deviceStatus).mockResolvedValue({...empty,receive:true,phase:"pending",unapplied:2,connection:{state_dir:"/queue",token_file:"/token",enabled:false,interval_seconds:300,allow_metered:false,allow_battery:false,capture_saved:false}});
  show();const apply=await screen.findByRole("button",{name:"Apply received files"});expect(apply).toBeDisabled();fireEvent.click(apply);expect(ipc.deviceApply).not.toHaveBeenCalled();
});

it("requires a separate opt-in to capture saved receiver edits",async()=>{
  vi.mocked(ipc.deviceStatus).mockResolvedValue({...empty,receive:true,connection:{state_dir:"/queue",token_file:"/token",enabled:false,interval_seconds:300,allow_metered:false,allow_battery:false,capture_saved:false}});
  show();
  const capture=await screen.findByRole("checkbox",{name:/Publish saved edits/});
  expect(capture).not.toBeChecked();
  fireEvent.click(capture);
  fireEvent.click(screen.getByRole("button",{name:"Save transfer settings"}));
  await waitFor(()=>expect(ipc.deviceConfigure).toHaveBeenCalledWith(expect.objectContaining({capture_saved:true,enabled:false})));
  expect(ipc.deviceRun).not.toHaveBeenCalled();
});
