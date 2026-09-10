// @vitest-environment jsdom
import {cleanup,render,screen,waitFor} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {afterEach,expect,it,vi} from "vitest";
import {SearchPanel} from "./SearchPanel";
import * as ipc from "../ipc";
vi.mock("../ipc",async original=>({...await original<typeof import("../ipc")>(),searchStart:vi.fn(),searchCancel:vi.fn(async()=>{}),searchPoll:vi.fn()}));
afterEach(()=>{cleanup();vi.clearAllMocks();});
it("cancels a search whose start arrives after the panel was closed",async()=>{
 let resolve!:(id:number)=>void;vi.mocked(ipc.searchStart).mockImplementation(()=>new Promise(r=>{resolve=r;}));
 const user=userEvent.setup();const {unmount}=render(<SearchPanel onClose={()=>{}}/>);
 await user.type(screen.getByRole("textbox"),"query{Enter}");unmount();resolve(42);
 await waitFor(()=>expect(ipc.searchCancel).toHaveBeenCalledWith(42));expect(ipc.searchPoll).not.toHaveBeenCalled();
});
it("keeps words separate from literal and regex with no case-sensitive promise",async()=>{
 const user=userEvent.setup();render(<SearchPanel onClose={()=>{}}/>);
 await user.selectOptions(screen.getByRole("combobox"),"words");expect(screen.getByRole("checkbox")).toBeDisabled();
 await user.selectOptions(screen.getByRole("combobox"),"regex");expect(screen.getByRole("checkbox")).toBeEnabled();
});
