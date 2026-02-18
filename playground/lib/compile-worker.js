import { sb3_to_wasm, WasmFlags } from "../../js/compiler/hyperquark.js";

console.log("web worker initialised");

postMessage("ready");

addEventListener("message", ({ data }) => {
  console.log("message received");
  let wasmProject = sb3_to_wasm(data.proj, WasmFlags.from_js(data.flags));
  postMessage({
    wasm_bytes: wasmProject.wasm_bytes,
    strings: wasmProject.strings,
    target_names: wasmProject.target_names,
  });
});