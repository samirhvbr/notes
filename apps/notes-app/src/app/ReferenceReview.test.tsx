// @vitest-environment jsdom
import {act,cleanup,render,screen,waitFor} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {afterEach,beforeEach,expect,it,vi} from "vitest";
import {ReferenceReview,reviewedMove} from "./ReferenceReview";
import * as ipc from "../ipc";
import {useWorkspace} from "../stores/workspace";
import {useEditor} from "../stores/editor";
import {askConfirm} from "./dialog";
vi.mock("./dialog",()=>({askConfirm:vi.fn(async()=>true)}));
vi.mock("../ipc",async original=>({...await original<typeof import("../ipc")>(),indexStart:vi.fn(async()=>({running:false})),referencePreview:vi.fn(),referenceApply:vi.fn(async()=>({path:"c.md",failed:[]})),entryRename:vi.fn(async()=>({path:"c.md"}))}));
beforeEach(()=>{
 useWorkspace.setState({info:{id:"ws"} as ipc.WorkspaceInfo});
 useEditor.setState({doc:null,save:vi.fn(async()=>{})});
 vi.mocked(ipc.referencePreview).mockResolvedValue({token:"test",from:"b.md",to:"c.md",skipped:0,files:[{path:"a.md",destination:"a.md",edits:[{start:1,end:5,before:"b.md",after:"c.md"}]}]} as ipc.ReferencePlan);
});
afterEach(()=>{cleanup();vi.clearAllMocks();});
it("applies only references selected in the review",async()=>{
 const user=userEvent.setup();render(<ReferenceReview/>);
 let work:ReturnType<typeof reviewedMove>;
 await act(async()=>{work=reviewedMove("b.md" as ipc.RelPath,"c.md" as ipc.RelPath);});
 await user.click(await screen.findByRole("checkbox"));
 await user.click(screen.getByRole("button",{name:/Apply/i}));
 await work!;
 expect(ipc.referenceApply).toHaveBeenCalledWith("test",[]);
});
it("Escape cancels without mutating the workspace",async()=>{
 const user=userEvent.setup();render(<ReferenceReview/>);
 let work:ReturnType<typeof reviewedMove>;
 await act(async()=>{work=reviewedMove("b.md" as ipc.RelPath,"c.md" as ipc.RelPath);});
 await screen.findByRole("dialog");await waitFor(()=>expect(document.activeElement).toBe(screen.getByRole("checkbox")));await user.keyboard("{Escape}");
 expect(await work!).toBeNull();expect(ipc.referenceApply).not.toHaveBeenCalled();
});
it("offers an explicit move without links when indexing fails",async()=>{
 vi.mocked(ipc.indexStart).mockRejectedValueOnce(new Error("unavailable"));
 await reviewedMove("b.md" as ipc.RelPath,"c.md" as ipc.RelPath);
 expect(askConfirm).toHaveBeenCalled();expect(ipc.entryRename).toHaveBeenCalledWith("b.md","c.md");expect(ipc.referenceApply).not.toHaveBeenCalled();
});
