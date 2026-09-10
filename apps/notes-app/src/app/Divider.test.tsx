// @vitest-environment jsdom
import {cleanup,fireEvent,render,screen} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {useRef} from "react";
import {afterEach,expect,it} from "vitest";
import {Divider} from "./Divider";
function Harness(){const panes=useRef<HTMLDivElement>(null);return <div ref={panes}><Divider panes={panes}/></div>;}
afterEach(cleanup);
it("supports keyboard resizing, limits and reset",async()=>{
 const user=userEvent.setup();render(<Harness/>);const divider=screen.getByRole("separator");await user.tab();expect(divider).toHaveFocus();
 await user.keyboard("{ArrowRight}");expect(divider).toHaveAttribute("aria-valuenow","52");
 await user.keyboard("{End}{ArrowRight}");expect(divider).toHaveAttribute("aria-valuenow","80");
 await user.keyboard("{Home}{ArrowLeft}");expect(divider).toHaveAttribute("aria-valuenow","20");
 await user.keyboard("{Enter}");expect(divider).toHaveAttribute("aria-valuenow","50");
});
it("cleans up drag styling if the split is closed during a drag",()=>{
 const {unmount}=render(<Harness/>);fireEvent.mouseDown(screen.getByRole("separator"));expect(document.body.style.userSelect).toBe("none");unmount();expect(document.body.style.userSelect).toBe("");expect(document.body.style.cursor).toBe("");
});
