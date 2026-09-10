// @vitest-environment jsdom
import {cleanup,render,screen} from "@testing-library/react";
import {afterEach,expect,it,vi} from "vitest";
import {Palette} from "./Palette";
import * as ipc from "../ipc";
vi.mock("../ipc",async original=>({...await original<typeof import("../ipc")>(),quickOpen:vi.fn()}));
afterEach(()=>{cleanup();vi.clearAllMocks();});
it("refreshes quick open when its initial path index is still building",async()=>{
 vi.mocked(ipc.quickOpen).mockResolvedValueOnce({matches:[],indexed:0,building:true,unreadable:0}).mockResolvedValue({matches:[{name:"a.md",path:"a.md",score:1}],indexed:1,building:false,unreadable:0});
 render(<Palette mode="files" commands={[]} onClose={()=>{}}/>);
 expect(await screen.findByText("a.md")).toBeInTheDocument();expect(ipc.quickOpen).toHaveBeenCalledTimes(2);
});
