/*export * from "./node_modules/asdom/assembly/glue";

import { document } from "./node_modules/asdom/assembly/index";
const el = document.createElement("h1");

el.setAttribute("foo", "bar");

const s: string = el.getAttribute("foo")!; // returns "bar"
*/
/*el.innerHTML = /*html*//* `
  <span style="font-weight: normal;">
    <em>hello</em> from <strong>AssemblyScript</strong>
  </span>
`;*/

//document.body!.appendChild(el);
export function add(a: i32, b: i32): i32 {
  return a + b;
}
